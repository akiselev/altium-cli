//! Color constants for the PCB viewer.

use eframe::egui::Color32;

pub const BACKGROUND: Color32 = Color32::from_rgb(30, 30, 30);
pub const BOARD_OUTLINE: Color32 = Color32::from_rgb(200, 200, 100);
pub const BOARD_FILL: Color32 = Color32::from_rgb(40, 50, 30);
pub const TOP_COMPONENT: Color32 = Color32::from_rgb(200, 60, 60);
pub const TOP_COMPONENT_FILL: Color32 = Color32::from_rgba_premultiplied(200, 60, 60, 40);
pub const BOTTOM_COMPONENT: Color32 = Color32::from_rgb(60, 60, 200);
pub const BOTTOM_COMPONENT_FILL: Color32 = Color32::from_rgba_premultiplied(60, 60, 200, 40);
pub const PAD_SMD: Color32 = Color32::from_rgb(100, 200, 100);
pub const PAD_TH: Color32 = Color32::from_rgb(220, 200, 60);
pub const VIA: Color32 = Color32::from_rgb(180, 180, 180);
pub const KEEPOUT_FILL: Color32 = Color32::from_rgba_premultiplied(180, 30, 30, 60);
pub const KEEPOUT_STROKE: Color32 = Color32::from_rgb(220, 40, 40);

pub fn layer_color(name: &str) -> Color32 {
    match name {
        "Top Layer" => Color32::from_rgb(200, 50, 50),
        "Bottom Layer" => Color32::from_rgb(50, 50, 200),
        "Mid Layer 1" => Color32::from_rgb(200, 200, 50),
        "Mid Layer 2" => Color32::from_rgb(50, 200, 50),
        "Mid Layer 3" => Color32::from_rgb(200, 150, 50),
        "Mid Layer 4" => Color32::from_rgb(150, 50, 200),
        "Mid Layer 5" => Color32::from_rgb(50, 200, 200),
        "Mid Layer 6" => Color32::from_rgb(200, 50, 150),
        _ if name.contains("Overlay") => Color32::from_rgb(220, 220, 160),
        _ if name.contains("Solder") => Color32::from_rgb(80, 140, 80),
        _ if name.contains("Paste") => Color32::from_rgb(130, 130, 130),
        _ => Color32::from_rgb(150, 150, 150),
    }
}

pub fn layer_color_alpha(name: &str, alpha: u8) -> Color32 {
    let c = layer_color(name);
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}
pub const RATSNEST: Color32 = Color32::from_rgba_premultiplied(100, 100, 255, 80);
pub const SELECTED: Color32 = Color32::from_rgb(255, 255, 0);
pub const TEXT_COLOR: Color32 = Color32::from_rgb(200, 200, 200);
