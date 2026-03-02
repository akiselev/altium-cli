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
pub const TRACK: Color32 = Color32::from_rgb(200, 50, 50);
pub const VIA: Color32 = Color32::from_rgb(180, 180, 180);
pub const RATSNEST: Color32 = Color32::from_rgba_premultiplied(100, 100, 255, 80);
pub const SELECTED: Color32 = Color32::from_rgb(255, 255, 0);
pub const TEXT_COLOR: Color32 = Color32::from_rgb(200, 200, 200);
