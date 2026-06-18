use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Fixed-point coordinate: 10,000 internal units = 1 mil (0.001 inch).
///
/// 1 mil = 10,000 units
/// 1 mm  = ~393,701 units
/// 1 inch = 10,000,000 units
/// Range: approximately +/- 214,748 mils = +/- 5,454 mm
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Coord(i32);

impl Coord {
    pub const ZERO: Self = Self(0);
    pub const ONE_MIL: Self = Self(10_000);
    pub const ONE_INCH: Self = Self(10_000_000);
    /// Nearest integer approximation: 1 mm = 39370.0787... mils * 10000 = 393700.787...
    /// Altium uses 393701.
    pub const ONE_MM: Self = Self(393_701);

    /// Maximum reasonable dimension for a single PCB feature (100 inches = 2,540 mm).
    ///
    /// Altium's maximum board size is ~100" x 100". No single feature dimension
    /// (via diameter, track width, mask expansion) should approach this. Meanwhile,
    /// binary parsing misalignment produces values near `i32::MAX` (~2.1 billion).
    /// This threshold catches all garbage while leaving a 2x margin before overflow.
    ///
    /// Used by invariant checks to detect binary parsing misalignment.
    pub const MAX_REASONABLE_DIMENSION: Self = Self(1_000_000_000);

    /// Units per mil (the fundamental resolution).
    pub const UNITS_PER_MIL: i32 = 10_000;
    /// DXP base unit for schematic parameter encoding.
    /// Each "DXP unit" (the integer part of LOCATION.X etc.) represents 10 mils.
    /// Source: Rt_Schematic.Consts.cBaseUnit = 100000.
    pub const DXP_BASE_UNIT: i32 = 100_000;

    pub fn from_mils(mils: i32) -> Option<Self> {
        mils.checked_mul(10_000).map(Self)
    }

    pub fn from_mils_f64(mils: f64) -> Self {
        Self((mils * 10_000.0).round() as i32)
    }

    pub fn from_mms(mm: f64) -> Self {
        // Altium uses 393701 internal units per mm.
        Self((mm * 393_701.0).round() as i32)
    }

    pub fn to_mils(self) -> f64 {
        self.0 as f64 / 10_000.0
    }

    pub fn to_mms(self) -> f64 {
        self.0 as f64 / 393_700.787_401_574_8
    }

    pub fn from_internal(raw: i32) -> Self {
        Self(raw)
    }

    pub fn to_internal(self) -> i32 {
        self.0
    }

    /// Alias for `from_internal`.
    pub fn new(raw: i32) -> Self {
        Self(raw)
    }

    /// Alias for `to_internal`.
    pub fn raw(self) -> i32 {
        self.0
    }

    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Construct from DXP integer + fractional parts.
    /// Each DXP integer unit = DXP_BASE_UNIT internal units (100,000 = 10 mils).
    /// The `frac` component stores the sub-unit remainder in internal units.
    pub fn from_dxp_frac(integer: i32, frac: i32) -> Self {
        Self(integer * Self::DXP_BASE_UNIT + frac)
    }

    /// Split into DXP integer + fractional parts using Euclidean division.
    /// Returns (integer_part, fractional_remainder) where 0 <= frac < DXP_BASE_UNIT.
    pub fn to_dxp_frac(self) -> (i32, i32) {
        (
            self.0.div_euclid(Self::DXP_BASE_UNIT),
            self.0.rem_euclid(Self::DXP_BASE_UNIT),
        )
    }

    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }
}

impl Add for Coord {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Coord {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Neg for Coord {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Mul<i32> for Coord {
    type Output = Self;
    fn mul(self, rhs: i32) -> Self {
        Self(self.0 * rhs)
    }
}

impl Div<i32> for Coord {
    type Output = Self;
    fn div(self, rhs: i32) -> Self {
        Self(self.0 / rhs)
    }
}

impl AddAssign for Coord {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Coord {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl fmt::Display for Coord {
    /// Format as the most natural unit for the spec language.
    ///
    /// Invariant: the printed string MUST parse back (via the spec-language
    /// unit conversions `from_mms`/`from_mils_f64`) to this exact coordinate.
    /// The mil form is always exact (4 decimals = 1 internal unit), so mm is
    /// only chosen when the printed mm string round-trips through `from_mms`
    /// (which uses Altium's 393,701 units/mm approximation, NOT the exact
    /// 0.0254 mm/mil ratio).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mils = self.to_mils();
        if self.0 != 0 {
            let mm_str = format_float_trimmed(mils * 0.0254);
            let decimals = mm_str.split('.').nth(1).map_or(0, str::len);
            if decimals <= 3 {
                if let Ok(parsed) = mm_str.parse::<f64>() {
                    if Self::from_mms(parsed) == *self {
                        return write!(f, "{}mm", mm_str);
                    }
                }
            }
        }
        write!(f, "{}mil", format_float_trimmed(mils))
    }
}

/// Format a float with up to 4 decimal places, stripping trailing zeros.
fn format_float_trimmed(v: f64) -> String {
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

/// 2D point with X and Y coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CoordPoint {
    pub x: Coord,
    pub y: Coord,
}

impl CoordPoint {
    pub fn new(x: Coord, y: Coord) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self {
            x: Coord::ZERO,
            y: Coord::ZERO,
        }
    }
}

impl fmt::Display for CoordPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

/// Axis-aligned bounding box defined by min and max corners.
/// Invariant: min.x <= max.x && min.y <= max.y (enforced at construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoundingBox {
    min: CoordPoint,
    max: CoordPoint,
}

impl BoundingBox {
    /// Construct a BoundingBox, panicking if min > max in either axis.
    pub fn new(min: CoordPoint, max: CoordPoint) -> Self {
        assert!(min.x <= max.x, "BoundingBox: min.x > max.x");
        assert!(min.y <= max.y, "BoundingBox: min.y > max.y");
        Self { min, max }
    }

    pub fn min(&self) -> CoordPoint {
        self.min
    }

    pub fn max(&self) -> CoordPoint {
        self.max
    }

    pub fn width(&self) -> Coord {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> Coord {
        self.max.y - self.min.y
    }

    pub fn contains_point(&self, p: CoordPoint) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    pub fn union(&self, other: &Self) -> Self {
        let min = CoordPoint {
            x: if self.min.x <= other.min.x {
                self.min.x
            } else {
                other.min.x
            },
            y: if self.min.y <= other.min.y {
                self.min.y
            } else {
                other.min.y
            },
        };
        let max = CoordPoint {
            x: if self.max.x >= other.max.x {
                self.max.x
            } else {
                other.max.x
            },
            y: if self.max.y >= other.max.y {
                self.max.y
            } else {
                other.max.y
            },
        };
        Self { min, max }
    }

    /// Construct from two arbitrary points, auto-normalizing.
    pub fn from_points(a: CoordPoint, b: CoordPoint) -> Self {
        Self {
            min: CoordPoint {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
            max: CoordPoint {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
        }
    }

    /// Single-point bounding box.
    pub fn from_point(p: CoordPoint) -> Self {
        Self { min: p, max: p }
    }

    /// Bounding box from an iterator of points. Returns None if empty.
    pub fn from_iter(mut iter: impl Iterator<Item = CoordPoint>) -> Option<Self> {
        let first = iter.next()?;
        let mut bbox = Self::from_point(first);
        for p in iter {
            bbox = bbox.union_point(p);
        }
        Some(bbox)
    }

    /// Center point.
    pub fn center(&self) -> CoordPoint {
        CoordPoint {
            x: Coord::from_internal((self.min.x.to_internal() + self.max.x.to_internal()) / 2),
            y: Coord::from_internal((self.min.y.to_internal() + self.max.y.to_internal()) / 2),
        }
    }

    /// Expand bounding box by a margin on all sides.
    pub fn expand(&self, margin: Coord) -> Self {
        Self {
            min: CoordPoint {
                x: self.min.x - margin,
                y: self.min.y - margin,
            },
            max: CoordPoint {
                x: self.max.x + margin,
                y: self.max.y + margin,
            },
        }
    }

    /// Extend bounding box to include a point.
    pub fn union_point(&self, p: CoordPoint) -> Self {
        Self {
            min: CoordPoint {
                x: self.min.x.min(p.x),
                y: self.min.y.min(p.y),
            },
            max: CoordPoint {
                x: self.max.x.max(p.x),
                y: self.max.y.max(p.y),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Re-parse a displayed coordinate the way the spec-language evaluator does
    /// (`Unit::Mm` → from_mms, `Unit::Mil` → from_mils_f64).
    fn parse_display(s: &str) -> Coord {
        if let Some(mm) = s.strip_suffix("mm") {
            Coord::from_mms(mm.parse::<f64>().unwrap())
        } else if let Some(mil) = s.strip_suffix("mil") {
            Coord::from_mils_f64(mil.parse::<f64>().unwrap())
        } else {
            panic!("displayed coord has no unit suffix: {s}");
        }
    }

    #[test]
    fn display_mil_grid_value_stays_in_mils() {
        // 200 mil = 2,000,000 units. The "nice" mm form 5.08mm would re-parse
        // to 2,000,001 units (5.08 * 393,701), so it must NOT be used.
        assert_eq!(Coord::from_internal(2_000_000).to_string(), "200mil");
        assert_eq!(Coord::from_internal(12_000_000).to_string(), "1200mil");
    }

    #[test]
    fn display_mm_grid_value_uses_mm() {
        // Altium stores 2.54mm as round(2.54 * 393,701) = 1,000,001 units.
        assert_eq!(Coord::from_internal(1_000_001).to_string(), "2.54mm");
        assert_eq!(Coord::from_mms(5.08).to_string(), "5.08mm");
        assert_eq!(Coord::from_mms(1.0).to_string(), "1mm");
        assert_eq!(Coord::from_mms(-0.635).to_string(), "-0.635mm");
    }

    #[test]
    fn display_zero_is_mil() {
        assert_eq!(Coord::ZERO.to_string(), "0mil");
    }

    #[test]
    fn display_reparses_to_identical_coord() {
        // Mix of mil-grid, mm-grid, and arbitrary raw values.
        let raws = [
            0,
            1,
            -1,
            39,
            394,
            10_000,
            -10_000,
            2_000_000,
            2_000_001,
            1_000_000,
            1_000_001,
            12_000_000,
            12_000_006,
            393_701,
            -393_701,
            123_456_789,
            i32::MAX / 2,
            -i32::MAX / 2,
        ];
        for raw in raws {
            let c = Coord::from_internal(raw);
            let shown = c.to_string();
            assert_eq!(
                parse_display(&shown),
                c,
                "display `{shown}` of raw {raw} does not re-parse to itself"
            );
        }
    }
}
