//! RecordingCanvas and NullCanvas implementations for testing.

use super::canvas::{AltiumCanvas, Brush, DrawPoint, FontSpec, Pen, RenderTransform};

/// Records all draw calls for later inspection (useful in tests).
#[derive(Debug, Clone)]
pub enum DrawCall {
    Line { p1: DrawPoint, p2: DrawPoint, pen: Pen },
    Polyline { points: Vec<DrawPoint>, pen: Pen },
    Arc { center: DrawPoint, rx: f64, ry: f64, start_deg: f64, end_deg: f64, pen: Pen },
    Ellipse { center: DrawPoint, rx: f64, ry: f64, pen: Pen, fill: Option<Brush> },
    Rect { p1: DrawPoint, p2: DrawPoint, pen: Pen, fill: Option<Brush> },
    RoundedRect { p1: DrawPoint, p2: DrawPoint, rx: f64, ry: f64, pen: Pen, fill: Option<Brush> },
    Polygon { points: Vec<DrawPoint>, pen: Pen, fill: Option<Brush> },
    Bezier { ctrl_pts: Vec<DrawPoint>, pen: Pen },
    Text { text: String, pos: DrawPoint, angle_deg: f64, font: FontSpec, color: Pen },
    Image { p1: DrawPoint, p2: DrawPoint },
    PushTransform(RenderTransform),
    PopTransform,
    PushClip { p1: DrawPoint, p2: DrawPoint },
    PopClip,
}

/// Canvas that records all draw calls for inspection in tests.
#[derive(Debug, Default)]
pub struct RecordingCanvas {
    pub calls: Vec<DrawCall>,
}

impl RecordingCanvas {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AltiumCanvas for RecordingCanvas {
    fn draw_line(&mut self, p1: DrawPoint, p2: DrawPoint, pen: &Pen) {
        self.calls.push(DrawCall::Line { p1, p2, pen: pen.clone() });
    }
    fn draw_polyline(&mut self, points: &[DrawPoint], pen: &Pen) {
        self.calls.push(DrawCall::Polyline { points: points.to_vec(), pen: pen.clone() });
    }
    fn draw_arc(&mut self, center: DrawPoint, rx: f64, ry: f64, start_deg: f64, end_deg: f64, pen: &Pen) {
        self.calls.push(DrawCall::Arc { center, rx, ry, start_deg, end_deg, pen: pen.clone() });
    }
    fn draw_ellipse(&mut self, center: DrawPoint, rx: f64, ry: f64, pen: &Pen, fill: Option<&Brush>) {
        self.calls.push(DrawCall::Ellipse { center, rx, ry, pen: pen.clone(), fill: fill.cloned() });
    }
    fn draw_rect(&mut self, p1: DrawPoint, p2: DrawPoint, pen: &Pen, fill: Option<&Brush>) {
        self.calls.push(DrawCall::Rect { p1, p2, pen: pen.clone(), fill: fill.cloned() });
    }
    fn draw_rounded_rect(&mut self, p1: DrawPoint, p2: DrawPoint, rx: f64, ry: f64, pen: &Pen, fill: Option<&Brush>) {
        self.calls.push(DrawCall::RoundedRect { p1, p2, rx, ry, pen: pen.clone(), fill: fill.cloned() });
    }
    fn draw_polygon(&mut self, points: &[DrawPoint], pen: &Pen, fill: Option<&Brush>) {
        self.calls.push(DrawCall::Polygon { points: points.to_vec(), pen: pen.clone(), fill: fill.cloned() });
    }
    fn draw_bezier(&mut self, ctrl_pts: &[DrawPoint], pen: &Pen) {
        self.calls.push(DrawCall::Bezier { ctrl_pts: ctrl_pts.to_vec(), pen: pen.clone() });
    }
    fn draw_text(&mut self, text: &str, pos: DrawPoint, angle_deg: f64, font: &FontSpec, color: &Pen) {
        self.calls.push(DrawCall::Text { text: text.to_owned(), pos, angle_deg, font: font.clone(), color: color.clone() });
    }
    fn draw_image(&mut self, _data: &[u8], p1: DrawPoint, p2: DrawPoint) {
        self.calls.push(DrawCall::Image { p1, p2 });
    }
    fn push_transform(&mut self, t: &RenderTransform) {
        self.calls.push(DrawCall::PushTransform(t.clone()));
    }
    fn pop_transform(&mut self) {
        self.calls.push(DrawCall::PopTransform);
    }
    fn push_clip(&mut self, p1: DrawPoint, p2: DrawPoint) {
        self.calls.push(DrawCall::PushClip { p1, p2 });
    }
    fn pop_clip(&mut self) {
        self.calls.push(DrawCall::PopClip);
    }
}

/// A no-op canvas that discards all draw calls (for smoke-test rendering).
#[derive(Debug, Default)]
pub struct NullCanvas;

impl AltiumCanvas for NullCanvas {
    fn draw_line(&mut self, _p1: DrawPoint, _p2: DrawPoint, _pen: &Pen) {}
    fn draw_polyline(&mut self, _points: &[DrawPoint], _pen: &Pen) {}
    fn draw_arc(&mut self, _center: DrawPoint, _rx: f64, _ry: f64, _start_deg: f64, _end_deg: f64, _pen: &Pen) {}
    fn draw_ellipse(&mut self, _center: DrawPoint, _rx: f64, _ry: f64, _pen: &Pen, _fill: Option<&Brush>) {}
    fn draw_rect(&mut self, _p1: DrawPoint, _p2: DrawPoint, _pen: &Pen, _fill: Option<&Brush>) {}
    fn draw_rounded_rect(&mut self, _p1: DrawPoint, _p2: DrawPoint, _rx: f64, _ry: f64, _pen: &Pen, _fill: Option<&Brush>) {}
    fn draw_polygon(&mut self, _points: &[DrawPoint], _pen: &Pen, _fill: Option<&Brush>) {}
    fn draw_bezier(&mut self, _ctrl_pts: &[DrawPoint], _pen: &Pen) {}
    fn draw_text(&mut self, _text: &str, _pos: DrawPoint, _angle_deg: f64, _font: &FontSpec, _color: &Pen) {}
    fn draw_image(&mut self, _data: &[u8], _p1: DrawPoint, _p2: DrawPoint) {}
    fn push_transform(&mut self, _t: &RenderTransform) {}
    fn pop_transform(&mut self) {}
    fn push_clip(&mut self, _p1: DrawPoint, _p2: DrawPoint) {}
    fn pop_clip(&mut self) {}
}
