use std::fmt;

/// Win32 COLORREF: 0x00BBGGRR format stored as i32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color(i32);

impl Color {
    pub const BLACK: Self = Self(0x00000000);
    pub const WHITE: Self = Self(0x00FFFFFF);
    pub const RED: Self = Self(0x000000FF);
    pub const GREEN: Self = Self(0x0000FF00);
    pub const BLUE: Self = Self(0x00FF0000);

    pub const fn new(colorref: i32) -> Self {
        Self(colorref)
    }

    pub const fn raw(self) -> i32 {
        self.0
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self((b as i32) << 16 | (g as i32) << 8 | r as i32)
    }

    pub fn r(self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    pub fn g(self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }

    pub fn b(self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r(), self.g(), self.b())
    }
}
