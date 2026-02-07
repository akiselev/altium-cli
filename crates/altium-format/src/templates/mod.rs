//! Template system for creating Altium objects with smart defaults.
//!
//! Templates provide strongly-typed inputs for creating schematic components,
//! PCB footprints, and schematic document elements. They are designed for:
//!
//! - **LLM structured output**: All template types derive `schemars::JsonSchema`
//!   so JSON Schema can be exported for tool-calling and structured output.
//! - **Smart defaults**: Most fields are optional. When omitted, the template
//!   system infers sensible values from context (existing components, naming
//!   conventions, grid settings, etc.).
//! - **CLI integration**: Template inputs map directly to CLI arguments,
//!   making templates the canonical path for all object creation.
//!
//! # Architecture
//!
//! ```text
//! Template Input (JSON/CLI args)
//!       │
//!       ▼
//! TemplateContext (existing doc state)
//!       │
//!       ▼
//! Template::apply() → Altium records
//! ```
//!
//! # Example
//!
//! ```ignore
//! use altium_format::templates::schlib::SchComponentTemplate;
//!
//! let template = SchComponentTemplate {
//!     name: "LM358".to_string(),
//!     description: Some("Dual Op-Amp".to_string()),
//!     pins: vec![
//!         PinTemplate { designator: "1".into(), name: "OUT_A".into(), side: Some("left".into()), ..Default::default() },
//!         PinTemplate { designator: "8".into(), name: "VCC".into(), electrical: Some("power".into()), side: Some("top".into()), ..Default::default() },
//!     ],
//!     ..Default::default()
//! };
//! ```

pub mod pcblib;
pub mod schlib;
pub mod schdoc;

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// Shared types used across multiple template modules
// ═══════════════════════════════════════════════════════════════════════════

/// A coordinate value that accepts either a number (mils) or a string with units.
///
/// # Accepted formats
/// - Number: `100` (interpreted as mils)
/// - String: `"100mil"`, `"2.54mm"`, `"0.1in"`
///
/// When used in JSON, this allows flexible input:
/// ```json
/// { "x": 100, "y": "2.54mm" }
/// ```
#[derive(Debug, Clone)]
pub struct CoordInput(pub f64);

impl CoordInput {
    /// Get the value in mils.
    pub fn to_mils(&self) -> f64 {
        self.0
    }

    /// Convert to raw internal coordinate value (10000 units per mil).
    pub fn to_raw(&self) -> i32 {
        (self.0 * 10000.0).round() as i32
    }

    /// Create from mils.
    pub fn from_mils(mils: f64) -> Self {
        CoordInput(mils)
    }

    /// Create from mm.
    pub fn from_mm(mm: f64) -> Self {
        CoordInput(mm / 0.0254)
    }
}

impl Default for CoordInput {
    fn default() -> Self {
        CoordInput(0.0)
    }
}

impl Serialize for CoordInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl schemars::JsonSchema for CoordInput {
    fn schema_name() -> String {
        "CoordInput".to_string()
    }

    fn json_schema(_gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::*;
        Schema::Object(SchemaObject {
            metadata: Some(Box::new(Metadata {
                description: Some(
                    "Coordinate value: number (mils) or string with unit (e.g., \"100mil\", \"2.54mm\", \"0.1in\")".to_string(),
                ),
                ..Default::default()
            })),
            subschemas: Some(Box::new(SubschemaValidation {
                any_of: Some(vec![
                    Schema::Object(SchemaObject {
                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
                        ..Default::default()
                    }),
                    Schema::Object(SchemaObject {
                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                        ..Default::default()
                    }),
                ]),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}

impl<'de> Deserialize<'de> for CoordInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};

        struct CoordInputVisitor;

        impl<'de> Visitor<'de> for CoordInputVisitor {
            type Value = CoordInput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "a number (mils) or a string with unit (e.g., \"100mil\", \"2.54mm\")",
                )
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(CoordInput(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(CoordInput(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(CoordInput(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                parse_coord_string(value).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(CoordInputVisitor)
    }
}

/// Parse a string into a numeric value and optional unit suffix.
///
/// Returns `(value, unit_lowercase)` where unit is empty if no suffix.
fn parse_number_with_unit(s: &str) -> Result<(f64, String), String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty coordinate string".to_string());
    }

    // Find where the number ends and the unit begins
    let mut split_pos = s.len();
    for (i, c) in s.char_indices().rev() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            split_pos = i + c.len_utf8();
            break;
        }
    }

    let num_str = &s[..split_pos];
    let unit_str = s[split_pos..].trim().to_lowercase();

    let value: f64 = num_str
        .parse()
        .map_err(|e| format!("invalid number '{}': {}", num_str, e))?;

    Ok((value, unit_str))
}

/// Convert a value with a given unit to mils.
fn to_mils_with_unit(value: f64, unit: &str) -> Result<f64, String> {
    match unit {
        "" | "mil" | "mils" => Ok(value),
        "mm" => Ok(value / 0.0254),
        "in" | "inch" | "inches" => Ok(value * 1000.0),
        "cm" => Ok(value * 10.0 / 0.0254),
        _ => Err(format!("unknown unit '{}'", unit)),
    }
}

/// Convert a value with a given unit to mm.
fn to_mm_with_unit(value: f64, unit: &str) -> Result<f64, String> {
    match unit {
        "" | "mm" => Ok(value),
        "mil" | "mils" => Ok(value * 0.0254),
        "in" | "inch" | "inches" => Ok(value * 25.4),
        "cm" => Ok(value * 10.0),
        _ => Err(format!("unknown unit '{}'", unit)),
    }
}

/// Parse a coordinate string with units to mils.
fn parse_coord_string(s: &str) -> Result<CoordInput, String> {
    let (value, unit) = parse_number_with_unit(s)?;
    let mils = to_mils_with_unit(value, &unit)?;
    Ok(CoordInput(mils))
}

/// A coordinate value in millimeters, for PCB templates where mm is conventional.
///
/// Accepts a number (mm) or a string with units.
#[derive(Debug, Clone)]
pub struct MmInput(pub f64);

impl MmInput {
    /// Get the value in mm.
    pub fn to_mm(&self) -> f64 {
        self.0
    }

    /// Get the value in mils.
    pub fn to_mils(&self) -> f64 {
        self.0 / 0.0254
    }
}

impl Default for MmInput {
    fn default() -> Self {
        MmInput(0.0)
    }
}

impl Serialize for MmInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl schemars::JsonSchema for MmInput {
    fn schema_name() -> String {
        "MmInput".to_string()
    }

    fn json_schema(_gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::*;
        Schema::Object(SchemaObject {
            metadata: Some(Box::new(Metadata {
                description: Some(
                    "Dimension value: number (mm) or string with unit (e.g., \"2.54mm\", \"100mil\")".to_string(),
                ),
                ..Default::default()
            })),
            subschemas: Some(Box::new(SubschemaValidation {
                any_of: Some(vec![
                    Schema::Object(SchemaObject {
                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
                        ..Default::default()
                    }),
                    Schema::Object(SchemaObject {
                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                        ..Default::default()
                    }),
                ]),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}

impl<'de> Deserialize<'de> for MmInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};

        struct MmInputVisitor;

        impl<'de> Visitor<'de> for MmInputVisitor {
            type Value = MmInput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter
                    .write_str("a number (mm) or a string with unit (e.g., \"2.54mm\", \"100mil\")")
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(MmInput(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(MmInput(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(MmInput(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                parse_mm_string(value).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(MmInputVisitor)
    }
}

/// Parse a mm-default coordinate string.
fn parse_mm_string(s: &str) -> Result<MmInput, String> {
    let (value, unit) = parse_number_with_unit(s)?;
    let mm = to_mm_with_unit(value, &unit)?;
    Ok(MmInput(mm))
}

/// Hex color string (RRGGBB format), used across templates.
///
/// When serialized to Altium's internal format (0xBBGGRR), the bytes are
/// swapped automatically.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct HexColor(pub String);

impl HexColor {
    /// Convert to Altium's internal color format (0xBBGGRR integer).
    pub fn to_altium_color(&self) -> Result<i32, String> {
        let hex = self.0.trim_start_matches('#');
        if hex.len() != 6 {
            return Err(format!("color must be 6 hex digits, got '{}'", self.0));
        }
        let r =
            u8::from_str_radix(&hex[0..2], 16).map_err(|e| format!("invalid color: {}", e))?;
        let g =
            u8::from_str_radix(&hex[2..4], 16).map_err(|e| format!("invalid color: {}", e))?;
        let b =
            u8::from_str_radix(&hex[4..6], 16).map_err(|e| format!("invalid color: {}", e))?;
        Ok((b as i32) << 16 | (g as i32) << 8 | r as i32)
    }
}

impl Default for HexColor {
    fn default() -> Self {
        HexColor("000000".to_string())
    }
}

/// List all available template names.
pub fn list_templates() -> Vec<&'static str> {
    vec![
        "schlib-component",
        "pcblib-footprint",
        "schdoc-placement",
    ]
}

/// Get the JSON Schema for a named template.
pub fn get_template_schema(name: &str) -> Option<schemars::schema::RootSchema> {
    match name {
        "schlib-component" => {
            Some(schemars::schema_for!(schlib::SchComponentTemplate))
        }
        "pcblib-footprint" => {
            Some(schemars::schema_for!(pcblib::PcbFootprintTemplate))
        }
        "schdoc-placement" => {
            Some(schemars::schema_for!(schdoc::SchDocPlacementTemplate))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coord_input_mils() {
        let c: CoordInput = serde_json::from_str("100").unwrap();
        assert_eq!(c.to_mils(), 100.0);
    }

    #[test]
    fn test_coord_input_string_mil() {
        let c: CoordInput = serde_json::from_str("\"200mil\"").unwrap();
        assert_eq!(c.to_mils(), 200.0);
    }

    #[test]
    fn test_coord_input_string_mm() {
        let c: CoordInput = serde_json::from_str("\"2.54mm\"").unwrap();
        assert!((c.to_mils() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_mm_input_default_mm() {
        let m: MmInput = serde_json::from_str("1.27").unwrap();
        assert_eq!(m.to_mm(), 1.27);
    }

    #[test]
    fn test_mm_input_string_mil() {
        let m: MmInput = serde_json::from_str("\"100mil\"").unwrap();
        assert!((m.to_mm() - 2.54).abs() < 0.001);
    }

    #[test]
    fn test_hex_color() {
        let c = HexColor("FF0000".to_string());
        assert_eq!(c.to_altium_color().unwrap(), 0x0000FF); // BGR
    }

    #[test]
    fn test_list_templates() {
        let templates = list_templates();
        assert!(templates.contains(&"schlib-component"));
        assert!(templates.contains(&"pcblib-footprint"));
        assert!(templates.contains(&"schdoc-placement"));
    }

    #[test]
    fn test_get_template_schema() {
        for name in list_templates() {
            let schema = get_template_schema(name);
            assert!(schema.is_some(), "schema missing for template '{}'", name);
        }
    }
}
