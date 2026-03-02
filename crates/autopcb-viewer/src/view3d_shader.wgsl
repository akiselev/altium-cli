// 2.5D PCB viewer shader — simple vertex-color with basic lighting.

struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec3<f32>,
    _pad: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) color:    vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);

    // Lambertian: ambient 0.35 + diffuse from fixed light
    let n_dot_l = max(dot(normalize(in.normal), normalize(uniforms.light_dir)), 0.0);
    let light = 0.35 + 0.65 * n_dot_l;
    out.color = in.color * light;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
