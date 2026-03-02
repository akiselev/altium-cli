//! 2.5D wgpu-based PCB view.
//!
//! Renders a perspective-ish orthographic view of the PCB IR with coloured
//! layer planes, via boxes, and component bounding boxes.

use bytemuck::{Pod, Zeroable};
use eframe::egui;
use egui_wgpu::wgpu;
use wgpu::util::DeviceExt;

use autopcb_ir::{BoardSide, PcbIr};

// ---------------------------------------------------------------------------
// Camera
// ---------------------------------------------------------------------------

/// Orbit camera using orthographic projection.
pub struct Camera {
    /// Yaw in radians (rotation around Z axis).
    pub yaw: f32,
    /// Pitch in radians (tilt from vertical). Clamped to (5°, 85°).
    pub pitch: f32,
    /// Distance (half-width of the orthographic frustum) in mm.
    pub zoom: f32,
    /// World-space target the camera orbits around.
    pub target: [f32; 3],
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: std::f32::consts::FRAC_PI_4,  // 45°
            pitch: 0.5236,                       // 30°
            zoom: 80.0,                          // mm half-width
            target: [0.0, 0.0, 0.0],
        }
    }
}

impl Camera {
    /// Build a view-projection matrix (column-major, NDC depth 0..1).
    ///
    /// Uses an orthographic projection sized by `zoom` and an orbit
    /// eye position computed from yaw/pitch.
    pub fn view_proj(&self, aspect: f32) -> [[f32; 4]; 4] {
        // Eye position on a sphere of radius 1 (scaled by zoom * 2 later via ortho)
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let eye_dir = [cp * cy, cp * sy, sp];
        let dist = self.zoom * 2.5;
        let eye = [
            self.target[0] + eye_dir[0] * dist,
            self.target[1] + eye_dir[1] * dist,
            self.target[2] + eye_dir[2] * dist,
        ];

        let view = look_at(eye, self.target, [0.0, 0.0, 1.0]);
        let proj = ortho(
            -self.zoom * aspect,
            self.zoom * aspect,
            -self.zoom,
            self.zoom,
            -dist * 4.0,
            dist * 4.0,
        );
        mat4_mul(proj, view)
    }

    /// Handle mouse drag (dx, dy in pixels, sensitivity in radians/pixel).
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        let sensitivity = 0.005;
        self.yaw -= dx * sensitivity;
        self.pitch = (self.pitch + dy * sensitivity)
            .clamp(0.087, 1.484); // 5° – 85°
    }

    /// Handle scroll wheel (positive = zoom in).
    pub fn scroll(&mut self, delta: f32) {
        self.zoom = (self.zoom * (1.0 - delta * 0.001)).clamp(1.0, 5000.0);
    }
}

// ---------------------------------------------------------------------------
// Vertex layout
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal:   [f32; 3],
    pub color:    [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x3,
    ];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// ---------------------------------------------------------------------------
// Uniform buffer
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 3],
    _pad:      f32,
}

// ---------------------------------------------------------------------------
// Mesh builder helpers
// ---------------------------------------------------------------------------

struct MeshBuilder {
    vertices: Vec<Vertex>,
    indices:  Vec<u32>,
}

impl MeshBuilder {
    fn new() -> Self {
        Self { vertices: Vec::new(), indices: Vec::new() }
    }

    /// Push a quad (two triangles) given 4 corner positions and a flat color.
    /// Corners must be in order: bottom-left, bottom-right, top-right, top-left (CCW).
    fn push_quad(&mut self, corners: [[f32; 3]; 4], normal: [f32; 3], color: [f32; 3]) {
        let base = self.vertices.len() as u32;
        for &pos in &corners {
            self.vertices.push(Vertex { position: pos, normal, color });
        }
        // Two triangles (CCW winding)
        self.indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
    }

    /// Push all 6 faces of an axis-aligned box.
    fn push_box(&mut self, min: [f32; 3], max: [f32; 3], color: [f32; 3]) {
        let [x0, y0, z0] = min;
        let [x1, y1, z1] = max;

        // Top face (+Z)
        self.push_quad(
            [[x0,y0,z1],[x1,y0,z1],[x1,y1,z1],[x0,y1,z1]],
            [0.0, 0.0, 1.0], color,
        );
        // Bottom face (-Z)
        self.push_quad(
            [[x0,y1,z0],[x1,y1,z0],[x1,y0,z0],[x0,y0,z0]],
            [0.0, 0.0, -1.0], color,
        );
        // Front face (+Y)
        self.push_quad(
            [[x0,y1,z0],[x1,y1,z0],[x1,y1,z1],[x0,y1,z1]],
            [0.0, 1.0, 0.0], color,
        );
        // Back face (-Y)
        self.push_quad(
            [[x0,y0,z1],[x1,y0,z1],[x1,y0,z0],[x0,y0,z0]],
            [0.0, -1.0, 0.0], color,
        );
        // Right face (+X)
        self.push_quad(
            [[x1,y0,z0],[x1,y1,z0],[x1,y1,z1],[x1,y0,z1]],
            [1.0, 0.0, 0.0], color,
        );
        // Left face (-X)
        self.push_quad(
            [[x0,y0,z1],[x0,y1,z1],[x0,y1,z0],[x0,y0,z0]],
            [-1.0, 0.0, 0.0], color,
        );
    }

    /// Push a flat horizontal slab (thin box extruded upward from z_base).
    fn push_slab(
        &mut self,
        x0: f32, y0: f32,
        x1: f32, y1: f32,
        z_base: f32, thickness: f32,
        color: [f32; 3],
    ) {
        self.push_box(
            [x0.min(x1), y0.min(y1), z_base],
            [x0.max(x1), y0.max(y1), z_base + thickness],
            color,
        );
    }
}

// ---------------------------------------------------------------------------
// Layer-to-Z mapping
// ---------------------------------------------------------------------------

const BOARD_THICKNESS_MM: f32 = 1.6;
const COPPER_THICKNESS_MM: f32 = 0.035;

/// Return Z position of a named copper layer's top surface.
fn layer_z(name: &str, n_copper: usize) -> f32 {
    if name.contains("Top") || n_copper <= 1 {
        BOARD_THICKNESS_MM
    } else if name.contains("Bottom") {
        0.0
    } else {
        // Inner layers: linearly interpolated between top and bottom
        // extract index from "Mid Layer N"
        let idx: usize = name
            .split_whitespace()
            .last()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let frac = idx as f32 / (n_copper.saturating_sub(1).max(1) as f32);
        BOARD_THICKNESS_MM * (1.0 - frac)
    }
}

/// RGB color for a layer name.
fn layer_rgb(name: &str) -> [f32; 3] {
    match name {
        "Top Layer"    => [0.8, 0.2, 0.2],
        "Bottom Layer" => [0.2, 0.2, 0.8],
        "Mid Layer 1"  => [0.8, 0.8, 0.2],
        "Mid Layer 2"  => [0.2, 0.8, 0.2],
        "Mid Layer 3"  => [0.8, 0.6, 0.2],
        "Mid Layer 4"  => [0.6, 0.2, 0.8],
        "Mid Layer 5"  => [0.2, 0.8, 0.8],
        "Mid Layer 6"  => [0.8, 0.2, 0.6],
        _              => [0.6, 0.6, 0.6],
    }
}

// ---------------------------------------------------------------------------
// Scene
// ---------------------------------------------------------------------------

/// GPU resources for the 3-D PCB scene.
pub struct PcbScene3D {
    pipeline:       wgpu::RenderPipeline,
    vertex_buffer:  wgpu::Buffer,
    index_buffer:   wgpu::Buffer,
    index_count:    u32,
    uniform_buffer: wgpu::Buffer,
    bind_group:     wgpu::BindGroup,
}

impl PcbScene3D {
    /// Build a scene from the PCB IR.
    pub fn from_ir(
        ir: &PcbIr,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let mut mesh = MeshBuilder::new();
        let n_copper = ir.layer_stack.copper_layer_count;

        // ── Board substrate ─────────────────────────────────────────────
        let b = &ir.board.bounds;
        let (bx0, by0) = (b.min.x as f32, b.min.y as f32);
        let (bx1, by1) = (b.max.x as f32, b.max.y as f32);
        // FR4 green
        mesh.push_box(
            [bx0, by0, 0.0],
            [bx1, by1, BOARD_THICKNESS_MM],
            [0.15, 0.35, 0.15],
        );

        // ── Tracks ──────────────────────────────────────────────────────
        for track in &ir.free_copper.tracks {
            let z = layer_z(&track.layer_name, n_copper);
            let color = layer_rgb(&track.layer_name);
            let half_w = (track.width_mm as f32) / 2.0;

            let (x0, y0) = (track.start.x as f32, track.start.y as f32);
            let (x1, y1) = (track.end.x   as f32, track.end.y   as f32);

            // Build an AABB that encompasses the track + half-width on each side
            let (lx0, lx1) = (x0.min(x1) - half_w, x0.max(x1) + half_w);
            let (ly0, ly1) = (y0.min(y1) - half_w, y0.max(y1) + half_w);

            mesh.push_slab(lx0, ly0, lx1, ly1, z, COPPER_THICKNESS_MM, color);
        }

        // ── Fills ────────────────────────────────────────────────────────
        for fill in &ir.free_copper.fills {
            let z = layer_z(&fill.layer_name, n_copper);
            let color = layer_rgb(&fill.layer_name);
            let (fx0, fy0) = (fill.corner1.x as f32, fill.corner1.y as f32);
            let (fx1, fy1) = (fill.corner2.x as f32, fill.corner2.y as f32);
            mesh.push_slab(fx0, fy0, fx1, fy1, z, COPPER_THICKNESS_MM, color);
        }

        // ── Polygons ─────────────────────────────────────────────────────
        for (_id, poly) in ir.polygons.iter() {
            if poly.vertices.len() < 3 {
                continue;
            }
            let z = layer_z(&poly.layer_name, n_copper);
            let color = layer_rgb(&poly.layer_name);

            // Fan triangulation from first vertex
            let verts: Vec<[f32; 2]> = poly.vertices.iter()
                .map(|p| [p.x as f32, p.y as f32])
                .collect();
            let _v0 = verts[0];
            let base = mesh.vertices.len() as u32;
            let normal = [0.0f32, 0.0, 1.0];

            // Push all vertices at z (top face)
            for &[x, y] in &verts {
                mesh.vertices.push(Vertex {
                    position: [x, y, z + COPPER_THICKNESS_MM],
                    normal,
                    color,
                });
            }
            // Fan triangles
            let n = verts.len() as u32;
            for i in 1..(n - 1) {
                mesh.indices.extend_from_slice(&[base, base + i, base + i + 1]);
            }
        }

        // ── Vias ─────────────────────────────────────────────────────────
        for via in &ir.free_copper.vias {
            let r = (via.diameter_mm as f32) / 2.0;
            let cx = via.position.x as f32;
            let cy = via.position.y as f32;
            // Draw as a box spanning the full board thickness
            mesh.push_box(
                [cx - r, cy - r, 0.0],
                [cx + r, cy + r, BOARD_THICKNESS_MM],
                [0.7, 0.7, 0.7],
            );
        }

        // ── Component bounding boxes ──────────────────────────────────────
        for (_id, comp) in ir.components.iter() {
            let color = match comp.side {
                BoardSide::Top    => [0.8, 0.25, 0.25],
                BoardSide::Bottom => [0.25, 0.25, 0.8],
            };
            let z_base = match comp.side {
                BoardSide::Top    => BOARD_THICKNESS_MM,
                BoardSide::Bottom => -(COPPER_THICKNESS_MM * 3.0),
            };
            let bb = &comp.world_bounds;
            mesh.push_box(
                [bb.min.x as f32, bb.min.y as f32, z_base],
                [bb.max.x as f32, bb.max.y as f32, z_base + COPPER_THICKNESS_MM * 3.0],
                color,
            );
        }

        // ── Upload to GPU ─────────────────────────────────────────────────
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("pcb3d_vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage:    wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("pcb3d_indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage:    wgpu::BufferUsages::INDEX,
        });
        let index_count = mesh.indices.len() as u32;

        // ── Uniform buffer ────────────────────────────────────────────────
        let uniforms = Uniforms {
            view_proj: [[0.0; 4]; 4],
            light_dir: [0.5, 0.5, 1.0],
            _pad:      0.0,
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("pcb3d_uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage:    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ── Bind group layout ─────────────────────────────────────────────
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pcb3d_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding:    0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty:         wgpu::BindingType::Buffer {
                    ty:                 wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size:   None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("pcb3d_bg"),
            layout:  &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding:  0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // ── Pipeline ──────────────────────────────────────────────────────
        let shader_src = include_str!("view3d_shader.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("pcb3d_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label:                Some("pcb3d_layout"),
            bind_group_layouts:   &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:  Some("pcb3d_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module:      &shader,
                entry_point: Some("vs_main"),
                buffers:     &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module:      &shader,
                entry_point: Some("fs_main"),
                targets:     &[Some(wgpu::ColorTargetState {
                    format:     target_format,
                    blend:      Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology:           wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face:         wgpu::FrontFace::Ccw,
                cull_mode:          Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format:              wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare:       wgpu::CompareFunction::Less,
                stencil:             Default::default(),
                bias:                Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask:  !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
            uniform_buffer,
            bind_group,
        }
    }
}

// ---------------------------------------------------------------------------
// egui_wgpu CallbackTrait integration
// ---------------------------------------------------------------------------

/// Stored in `egui_wgpu::CallbackResources` during initialisation.
pub struct SceneResources {
    pub scene: PcbScene3D,
}

/// State shared between egui's prepare and paint phases.
pub struct PcbScene3DCallback {
    pub camera: Camera,
    pub viewport_rect: egui::Rect,
}

impl egui_wgpu::CallbackTrait for PcbScene3DCallback {
    fn prepare(
        &self,
        _device:   &wgpu::Device,
        queue:     &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let res: &SceneResources = resources.get().unwrap();
        let aspect = self.viewport_rect.width() / self.viewport_rect.height().max(1.0);
        let vp = self.camera.view_proj(aspect);
        let uniforms = Uniforms {
            view_proj: vp,
            light_dir: [0.577, 0.577, 0.577],
            _pad:      0.0,
        };
        queue.write_buffer(
            &res.scene.uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );
        Vec::new()
    }

    fn paint(
        &self,
        _info:     egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let res: &SceneResources = resources.get().unwrap();
        let scene = &res.scene;
        render_pass.set_pipeline(&scene.pipeline);
        render_pass.set_bind_group(0, &scene.bind_group, &[]);
        render_pass.set_vertex_buffer(0, scene.vertex_buffer.slice(..));
        render_pass.set_index_buffer(scene.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..scene.index_count, 0, 0..1);
    }
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

fn look_at(eye: [f32; 3], center: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let f = normalize(sub3(center, eye));
    let s = normalize(cross3(f, up));
    let u = cross3(s, f);

    [
        [s[0],            u[0],            -f[0],           0.0],
        [s[1],            u[1],            -f[1],           0.0],
        [s[2],            u[2],            -f[2],           0.0],
        [-dot3(s, eye), -dot3(u, eye),  dot3(f, eye),  1.0],
    ]
}

/// Orthographic projection (right-handed, NDC depth 0..1 for wgpu).
fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let rml = right - left;
    let tmb = top   - bottom;
    let fmn = far   - near;
    [
        [2.0 / rml,            0.0,                    0.0,          0.0],
        [0.0,                  2.0 / tmb,              0.0,          0.0],
        [0.0,                  0.0,                   -1.0 / fmn,   0.0],
        [-(right+left)/rml,  -(top+bottom)/tmb,  -near/fmn,   1.0],
    ]
}

fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                out[i][j] += a[k][j] * b[i][k];
            }
        }
    }
    out
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0]-b[0], a[1]-b[1], a[2]-b[2]]
}
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0]*b[0] + a[1]*b[1] + a[2]*b[2]
}
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1]*b[2] - a[2]*b[1],
        a[2]*b[0] - a[0]*b[2],
        a[0]*b[1] - a[1]*b[0],
    ]
}
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = dot3(v, v).sqrt().max(1e-8);
    [v[0]/len, v[1]/len, v[2]/len]
}
