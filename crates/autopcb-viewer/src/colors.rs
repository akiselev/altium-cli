//! Color constants and default layer colors for the PCB viewer.

use eframe::egui::Color32;

pub const BACKGROUND: Color32 = Color32::from_rgb(30, 30, 30);
pub const BOARD_OUTLINE: Color32 = Color32::from_rgb(200, 200, 100);
pub const BOARD_FILL: Color32 = Color32::from_rgb(40, 50, 30);
pub const KEEPOUT_FILL: Color32 = Color32::from_rgba_premultiplied(180, 30, 30, 60);
pub const KEEPOUT_STROKE: Color32 = Color32::from_rgb(220, 40, 40);
pub const RATSNEST: Color32 = Color32::from_rgba_premultiplied(100, 100, 255, 80);
pub const SELECTED: Color32 = Color32::from_rgb(255, 255, 0);
pub const TEXT_COLOR: Color32 = Color32::from_rgb(200, 200, 200);
pub const TAB_FILL: Color32 = Color32::from_rgb(38, 38, 38);
pub const TAB_STROKE: Color32 = Color32::from_rgb(78, 78, 78);

pub const TOP_COMPONENT_LAYER: &str = "Top Components";
pub const BOTTOM_COMPONENT_LAYER: &str = "Bottom Components";
pub const MULTI_LAYER: &str = "Multi Layer";

fn mid_layer_color(index: usize) -> Color32 {
    const PALETTE: [Color32; 10] = [
        Color32::from_rgb(200, 200, 50),
        Color32::from_rgb(50, 200, 50),
        Color32::from_rgb(200, 150, 50),
        Color32::from_rgb(150, 50, 200),
        Color32::from_rgb(50, 200, 200),
        Color32::from_rgb(200, 50, 150),
        Color32::from_rgb(180, 110, 40),
        Color32::from_rgb(90, 180, 40),
        Color32::from_rgb(40, 180, 180),
        Color32::from_rgb(180, 80, 120),
    ];
    PALETTE[index % PALETTE.len()]
}

fn normalized_layer_name(name: &str) -> String {
    name.replace(['_', '-'], " ")
}

pub fn default_layer_color(name: &str) -> Color32 {
    let name = normalized_layer_name(name);
    match name.as_str() {
        "Top Layer" => Color32::from_rgb(200, 50, 50),
        "Bottom Layer" => Color32::from_rgb(50, 90, 220),
        "Top Components" => Color32::from_rgb(235, 110, 90),
        "Bottom Components" => Color32::from_rgb(110, 140, 235),
        "Multi Layer" => Color32::from_rgb(220, 200, 60),
        "Top Overlay" | "TopOverlay" => Color32::from_rgb(235, 235, 180),
        "Bottom Overlay" | "BottomOverlay" => Color32::from_rgb(170, 205, 235),
        "Top Paste" | "TopPaste" => Color32::from_rgb(170, 170, 170),
        "Bottom Paste" | "BottomPaste" => Color32::from_rgb(130, 150, 170),
        "Top Solder" | "TopSolder" | "Top Solder Mask" | "TopSolderMask" => {
            Color32::from_rgb(70, 145, 80)
        }
        "Bottom Solder" | "BottomSolder" | "Bottom Solder Mask" | "BottomSolderMask" => {
            Color32::from_rgb(55, 115, 120)
        }
        "Keep Out Layer" | "KeepOutLayer" | "Keepout" => Color32::from_rgb(220, 70, 70),
        _ if name.starts_with("Mid Layer ") => {
            let index = name
                .trim_start_matches("Mid Layer ")
                .parse::<usize>()
                .ok()
                .and_then(|n| n.checked_sub(1))
                .unwrap_or(0);
            mid_layer_color(index)
        }
        _ if name.starts_with("Mechanical") => Color32::from_rgb(170, 120, 200),
        _ if name.contains("Overlay") => Color32::from_rgb(220, 220, 160),
        _ if name.contains("Solder") => Color32::from_rgb(80, 140, 80),
        _ if name.contains("Paste") => Color32::from_rgb(130, 130, 130),
        _ => Color32::from_rgb(150, 150, 150),
    }
}

pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

pub fn layer_order_key(name: &str) -> (u8, u8, String) {
    let normalized = normalized_layer_name(name);
    match normalized.as_str() {
        "Top Layer" => (0, 0, normalized),
        "Top Components" => (1, 0, normalized),
        "Top Overlay" => (2, 0, normalized),
        "Top Paste" => (3, 0, normalized),
        "Top Solder" | "Top Solder Mask" => (4, 0, normalized),
        "Multi Layer" => (5, 0, normalized),
        "Bottom Solder" | "Bottom Solder Mask" => (7, 0, normalized),
        "Bottom Paste" => (8, 0, normalized),
        "Bottom Overlay" => (9, 0, normalized),
        "Bottom Components" => (10, 0, normalized),
        "Bottom Layer" => (11, 0, normalized),
        _ if normalized.starts_with("Mid Layer ") => {
            let index = normalized
                .trim_start_matches("Mid Layer ")
                .parse::<u8>()
                .unwrap_or(0);
            (6, index, normalized)
        }
        _ if normalized.starts_with("Mechanical") => (12, 0, normalized),
        _ => (13, 0, normalized),
    }
}
