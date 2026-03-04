use efame::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconId {
    Explorer,
    Search,
    SourceControl,
    Run,
    Extensions,
    File,
    Folder,
    PcbDoc,
    Spec,
    Gear,
    Close,
}

fn draw_icon(p: &Painter, rect: Rect, id: IconId, color: Color32) {
    let c = rect.center();
    let w = rect.width();
    let h = rect.height();
    let stroke = Stroke::new(1.5, color);
    match id {
        IconId::Explorer => {
            let r1 =
                Rect::from_center_size(c + Vec2::new(0.0, -h * 0.15), Vec2::new(w * 0.7, h * 0.45));
            let r2 =
                Rect::from_center_size(c + Vec2::new(0.0, h * 0.25), Vec2::new(w * 0.7, h * 0.45));
            p.rect_stroke(r1, 0.0, stroke, egui::StrokeKind::Outside);
            p.rect_stroke(r2, 0.0, stroke, egui::StrokeKind::Outside);
        }
        IconId::Search => {
            p.circle_stroke(c + Vec2::new(-w * 0.1, -h * 0.1), w * 0.25, stroke);
            p.line_segment(
                [
                    c + Vec2::new(w * 0.1, h * 0.1),
                    c + Vec2::new(w * 0.32, h * 0.32),
                ],
                stroke,
            );
        }
        IconId::SourceControl => {
            p.circle_filled(c + Vec2::new(-w * 0.22, -h * 0.18), w * 0.11, color);
            p.circle_filled(c + Vec2::new(w * 0.22, -h * 0.03), w * 0.11, color);
            p.circle_filled(c + Vec2::new(-w * 0.22, h * 0.24), w * 0.11, color);
            p.line_segment(
                [
                    c + Vec2::new(-w * 0.11, -h * 0.14),
                    c + Vec2::new(w * 0.11, -h * 0.05),
                ],
                stroke,
            );
            p.line_segment(
                [
                    c + Vec2::new(-w * 0.11, h * 0.18),
                    c + Vec2::new(w * 0.11, -h * 0.0),
                ],
                stroke,
            );
        }
        IconId::Run => {
            let pts = vec![
                c + Vec2::new(-w * 0.2, -h * 0.28),
                c + Vec2::new(w * 0.28, 0.0),
                c + Vec2::new(-w * 0.2, h * 0.28),
            ];
            p.add(Shape::closed_line(pts, stroke));
        }
        IconId::Extensions => {
            let size = Vec2::new(w * 0.26, h * 0.26);
            p.rect_stroke(
                Rect::from_center_size(c + Vec2::new(-w * 0.17, -h * 0.17), size),
                0.0,
                stroke,
                egui::StrokeKind::Outside,
            );
            p.rect_stroke(
                Rect::from_center_size(c + Vec2::new(w * 0.17, -h * 0.17), size),
                0.0,
                stroke,
                egui::StrokeKind::Outside,
            );
            p.rect_stroke(
                Rect::from_center_size(c + Vec2::new(-w * 0.17, h * 0.17), size),
                0.0,
                stroke,
                egui::StrokeKind::Outside,
            );
            p.rect_stroke(
                Rect::from_center_size(c + Vec2::new(w * 0.17, h * 0.17), size),
                0.0,
                stroke,
                egui::StrokeKind::Outside,
            );
        }
        IconId::File | IconId::PcbDoc | IconId::Spec => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(w * 0.6, h * 0.7)),
                0.0,
                stroke,
                egui::StrokeKind::Outside,
            );
            if matches!(id, IconId::PcbDoc) {
                p.text(
                    c + Vec2::new(0.0, h * 0.08),
                    Align2::CENTER_CENTER,
                    "P",
                    FontId::proportional(h * 0.32),
                    color,
                );
            } else if matches!(id, IconId::Spec) {
                p.text(
                    c + Vec2::new(0.0, h * 0.08),
                    Align2::CENTER_CENTER,
                    "S",
                    FontId::proportional(h * 0.32),
                    color,
                );
            }
        }
        IconId::Folder => {
            p.rect_stroke(
                Rect::from_min_size(
                    Pos2::new(rect.left() + w * 0.15, rect.top() + h * 0.28),
                    Vec2::new(w * 0.7, h * 0.45),
                ),
                0.0,
                stroke,
                egui::StrokeKind::Outside,
            );
            p.line_segment(
                [
                    Pos2::new(rect.left() + w * 0.17, rect.top() + h * 0.28),
                    Pos2::new(rect.left() + w * 0.38, rect.top() + h * 0.14),
                ],
                stroke,
            );
        }
        IconId::Gear => {
            p.circle_stroke(c, w * 0.22, stroke);
            p.circle_filled(c, w * 0.08, color);
        }
        IconId::Close => {
            p.line_segment(
                [
                    c + Vec2::new(-w * 0.2, -h * 0.2),
                    c + Vec2::new(w * 0.2, h * 0.2),
                ],
                stroke,
            );
            p.line_segment(
                [
                    c + Vec2::new(w * 0.2, -h * 0.2),
                    c + Vec2::new(-w * 0.2, h * 0.2),
                ],
                stroke,
            );
        }
    }
}

pub fn icon(ui: &mut egui::Ui, id: IconId, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    draw_icon(ui.painter(), rect, id, color);
}

pub fn icon_button(
    ui: &mut egui::Ui,
    id: IconId,
    selected: bool,
    tint: Color32,
    size: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if selected {
        ui.painter().rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 16),
        );
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 12),
        );
    }
    draw_icon(ui.painter(), rect.shrink(4.0), id, tint);
    response
}
