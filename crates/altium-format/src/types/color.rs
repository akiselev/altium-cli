//! Color type for Altium files.
//!
//! Altium stores colors as Win32 COLORREF values (BGR format).

use std::fmt;

/// Color type stored as Win32 COLORREF (BGR format).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// Black color.
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };

    /// White color.
    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
    };

    /// Red color.
    pub const RED: Color = Color { r: 255, g: 0, b: 0 };

    /// Green color.
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0 };

    /// Blue color.
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255 };

    /// Yellow color.
    pub const YELLOW: Color = Color {
        r: 255,
        g: 255,
        b: 0,
    };

    /// Creates a new color from RGB values.
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    /// Creates a color from a Win32 COLORREF value (stored as i32 in Altium).
    ///
    /// COLORREF is in BGR format: 0x00BBGGRR
    #[inline]
    pub fn from_win32(value: i32) -> Self {
        Color {
            r: (value & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: ((value >> 16) & 0xFF) as u8,
        }
    }

    /// Converts to a Win32 COLORREF value.
    #[inline]
    pub fn to_win32(self) -> i32 {
        (self.r as i32) | ((self.g as i32) << 8) | ((self.b as i32) << 16)
    }

    /// Creates a color from RGB hex value (0xRRGGBB).
    #[inline]
    pub fn from_rgb_hex(value: u32) -> Self {
        Color {
            r: ((value >> 16) & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: (value & 0xFF) as u8,
        }
    }

    /// Converts to RGB hex value.
    #[inline]
    pub fn to_rgb_hex(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

impl fmt::Debug for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Color(#{:02X}{:02X}{:02X})", self.r, self.g, self.b)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_win32_color() {
        // Red in BGR format is 0x0000FF
        let red = Color::from_win32(0x0000FF);
        assert_eq!(red, Color::RED);
        assert_eq!(red.to_win32(), 0x0000FF);

        // Blue in BGR format is 0xFF0000
        let blue = Color::from_win32(0xFF0000);
        assert_eq!(blue, Color::BLUE);
    }

    #[test]
    fn test_rgb_hex() {
        let red = Color::from_rgb_hex(0xFF0000);
        assert_eq!(red, Color::RED);
        assert_eq!(red.to_rgb_hex(), 0xFF0000);
    }
}
