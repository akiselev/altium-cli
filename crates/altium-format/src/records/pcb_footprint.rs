//! PCB Footprint (Component) record type for the v2 API.
//!
//! The footprint metadata uses pure parametric format (`|KEY=VALUE|` ASCII text).
//! Key fields: PATTERN, SOURCEDESIGNATOR, X/Y location, ROTATION, LAYER,
//! NAMEON, COMMENTON, GROUPNUM, HEIGHT, DESCRIPTION, etc.

use crate::coord::PcbCoord;
use altium_format_derive::altium_record;

#[altium_record(kind = "pcb", object_id = Component, codec = "params")]
pub struct PcbFootprintRecord {
    /// Footprint pattern name (e.g., "SOT-23", "DIP-8").
    #[altium(key = "PATTERN")]
    pattern: String,
    /// Component designator (e.g., "U1", "R1").
    #[altium(key = "SOURCEDESIGNATOR")]
    source_designator: String,
    /// Footprint description.
    #[altium(key = "DESCRIPTION")]
    description: String,
    /// X location in PCB coordinates.
    #[altium(key = "X")]
    location_x: PcbCoord,
    /// Y location in PCB coordinates.
    #[altium(key = "Y")]
    location_y: PcbCoord,
    /// Rotation angle in degrees.
    #[altium(key = "ROTATION")]
    rotation: f64,
    /// Layer (0-82).
    #[altium(key = "LAYER")]
    layer: u8,
    /// Component height.
    #[altium(key = "HEIGHT")]
    height: PcbCoord,
    /// Whether the name is visible.
    #[altium(key = "NAMEON")]
    name_on: bool,
    /// Whether the comment is visible.
    #[altium(key = "COMMENTON")]
    comment_on: bool,
    /// Group number.
    #[altium(key = "GROUPNUM")]
    group_num: i32,
    /// Component kind.
    #[altium(key = "COMPONENTKIND")]
    component_kind: i32,
    /// Unique identifier.
    #[altium(key = "UNIQUEID")]
    unique_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};
    use crate::coord::AltiumCoord;

    #[test]
    fn footprint_read_pattern() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|PATTERN=SOT-23|DESCRIPTION=Small transistor|SOURCEDESIGNATOR=Q1|",
        ));
        let rec = PcbFootprintRecord::from_origin(origin);

        assert_eq!(rec.pattern(), "SOT-23");
        assert_eq!(rec.description(), "Small transistor");
        assert_eq!(rec.source_designator(), "Q1");
    }

    #[test]
    fn footprint_read_coordinates() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|PATTERN=DIP-8|X=100000|Y=200000|ROTATION=90.000000|LAYER=1|HEIGHT=50000|",
        ));
        let rec = PcbFootprintRecord::from_origin(origin);

        assert_eq!(rec.location_x().to_raw(), 100_000);
        assert_eq!(rec.location_y().to_raw(), 200_000);
        assert!((rec.rotation() - 90.0).abs() < 1e-3);
        assert_eq!(rec.layer(), 1);
        assert_eq!(rec.height().to_raw(), 50_000);
    }

    #[test]
    fn footprint_read_flags() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|PATTERN=QFP-44|NAMEON=T|COMMENTON=F|"));
        let rec = PcbFootprintRecord::from_origin(origin);

        assert!(rec.name_on());
        assert!(!rec.comment_on());
    }

    #[test]
    fn footprint_write_roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|PATTERN=SOT-23|DESCRIPTION=Small transistor|",
        ));
        let mut rec = PcbFootprintRecord::from_origin(origin);

        rec.set_pattern("SOT-23-5".to_string());
        assert_eq!(rec.pattern(), "SOT-23-5");

        rec.set_location_x(PcbCoord::from_raw(500_000));
        assert_eq!(rec.location_x().to_raw(), 500_000);

        rec.set_rotation(45.0);
        assert!((rec.rotation() - 45.0).abs() < 1e-3);

        rec.set_name_on(true);
        assert!(rec.name_on());
    }

    #[test]
    fn footprint_defaults_for_missing() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|PATTERN=TEST|"));
        let rec = PcbFootprintRecord::from_origin(origin);

        // Missing fields should return defaults
        assert_eq!(rec.source_designator(), "");
        assert_eq!(rec.description(), "");
        assert_eq!(rec.location_x().to_raw(), 0);
        assert_eq!(rec.layer(), 0);
        assert!(!rec.name_on());
    }

    #[test]
    fn footprint_builder() {
        fn template() -> RecordOrigin {
            RecordOrigin::Param(ParamOrigin::new("|PATTERN=|"))
        }

        let rec = PcbFootprintRecord::builder(template)
            .pattern("BGA-256".to_string())
            .description("256-ball BGA package".to_string())
            .location_x(PcbCoord::from_raw(1_000_000))
            .location_y(PcbCoord::from_raw(2_000_000))
            .layer(1)
            .build();

        assert_eq!(rec.pattern(), "BGA-256");
        assert_eq!(rec.description(), "256-ball BGA package");
        assert_eq!(rec.location_x().to_raw(), 1_000_000);
        assert_eq!(rec.layer(), 1);
    }
}
