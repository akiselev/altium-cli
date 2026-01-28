//! Unit tests for schematic records.

use crate::records::sch::common::PinElectricalType;
use crate::records::sch::{SchComponent, SchPin, SchPrimitive};

#[test]
fn test_component_roundtrip() {
    let mut comp = SchComponent::default();
    comp.lib_reference = "TestRef".to_string();
    comp.part_count = 1;

    let params = comp.export_to_params();
    assert_eq!(params.get("LIBREFERENCE").unwrap().as_str(), "TestRef");

    let comp2 = SchComponent::import_from_params(&params).unwrap();
    assert_eq!(comp2.lib_reference, "TestRef");
    assert_eq!(comp2.part_count, 1);
}

#[test]
fn test_pin_roundtrip() {
    let mut pin = SchPin::default();
    pin.name = "Pin1".to_string();
    pin.electrical = PinElectricalType::Input;
    pin.pin_length = 3000000;

    let params = pin.export_to_params();
    assert_eq!(params.get("NAME").unwrap().as_str(), "Pin1");

    let pin2 = SchPin::import_from_params(&params).unwrap();
    assert_eq!(pin2.name, "Pin1");
    assert_eq!(pin2.pin_length, 3000000);
}

#[test]
fn test_m4_trait_extensions() {
    use crate::types::CoordPoint;

    // Test Pin
    let mut pin = SchPin::default();
    pin.graphical.location_x = 100;
    pin.graphical.location_y = 200;
    pin.name = "TestPin".to_string();
    pin.designator = "1".to_string();

    assert_eq!(pin.location(), Some(CoordPoint::from_raw(100, 200)));
    assert_eq!(pin.record_type_name(), "Pin");
    assert_eq!(pin.get_property("NAME"), Some("TestPin".to_string()));
    assert_eq!(pin.get_property("DESIGNATOR"), Some("1".to_string()));
    assert_eq!(pin.get_property("NONEXISTENT"), None);

    // Test Component
    let mut comp = SchComponent::default();
    comp.lib_reference = "RESISTOR".to_string();
    comp.graphical.location_x = 1000;
    comp.graphical.location_y = 2000;

    assert_eq!(comp.record_type_name(), "Component");
    assert_eq!(comp.location(), Some(CoordPoint::from_raw(1000, 2000)));
    assert_eq!(
        comp.get_property("LIBREFERENCE"),
        Some("RESISTOR".to_string())
    );
    assert_eq!(comp.get_property("NONEXISTENT"), None);
}

#[test]
fn test_polymorphic_access_via_schrecord() {
    use crate::records::sch::SchRecord;

    // Test that we can use trait methods polymorphically via SchRecord enum
    // This demonstrates that all 30 record types can be accessed uniformly

    let pin = SchPin::default();
    let mut comp = SchComponent::default();
    comp.lib_reference = "RESISTOR".to_string();

    let records: Vec<SchRecord> = vec![SchRecord::Pin(pin), SchRecord::Component(comp)];

    for record in &records {
        // Can call owner_index on SchRecord (polymorphic access)
        let _ = record.owner_index();

        // Can call the new trait methods polymorphically
        let _ = record.location();
        let type_name = record.record_type_name();
        assert!(!type_name.is_empty());

        // Verify type names
        match record {
            SchRecord::Pin(_) => assert_eq!(type_name, "Pin"),
            SchRecord::Component(_) => assert_eq!(type_name, "Component"),
            _ => {}
        }

        let _ = record.get_property("TEST");
    }

    // Verify specific property access
    if let SchRecord::Component(c) = &records[1] {
        assert_eq!(c.get_property("LIBREFERENCE"), Some("RESISTOR".to_string()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Regression tests for bugs found during HydroFlow schematic capture
// ═══════════════════════════════════════════════════════════════════════════

/// Regression: SchDesignator must serialize with RECORD=34, not RECORD=41.
///
/// SchDesignator flattens SchParameter (record_id=41). The derive macro's
/// append_to_params must write the parent's RECORD *after* flattened fields
/// so the parent's record_id wins.
#[test]
fn test_designator_roundtrip_record_id() {
    use crate::records::sch::designator::SchDesignator;
    use crate::records::sch::{SchRecord, SchParameter, SchLabel, SchGraphicalBase, SchPrimitiveBase};
    use crate::traits::ToParams;

    // Build a designator for "U1"
    let label = SchLabel {
        graphical: SchGraphicalBase {
            base: SchPrimitiveBase {
                owner_index: 0,
                ..Default::default()
            },
            location_x: 100,
            location_y: 200,
            ..Default::default()
        },
        text: "U1".to_string(),
        font_id: 1,
        ..Default::default()
    };

    let param = SchParameter {
        label,
        name: "Designator".to_string(),
        read_only_state: 1,
        ..Default::default()
    };

    let designator = SchDesignator {
        param,
        ..Default::default()
    };

    // Serialize to params
    let params = designator.to_params();

    // CRITICAL: RECORD must be 34 (Designator), NOT 41 (Parameter)
    let record_id = params.get("RECORD").expect("RECORD param must exist").as_int_or(-1);
    assert_eq!(
        record_id, 34,
        "SchDesignator must serialize as RECORD=34, got RECORD={}. \
         The derive macro flatten is likely overwriting parent record_id with child's.",
        record_id
    );

    // Roundtrip: parse back from params
    let parsed = SchRecord::from_params(&params).expect("Must parse back");
    match &parsed {
        SchRecord::Designator(d) => {
            assert_eq!(d.text(), "U1", "Designator text must survive roundtrip");
            assert_eq!(d.param.name, "Designator");
        }
        SchRecord::Parameter(_) => {
            panic!(
                "SchDesignator was parsed back as Parameter (RECORD=41). \
                 The flatten overwrite bug is present."
            );
        }
        other => {
            panic!("Expected Designator, got {:?}", other.record_type_name());
        }
    }
}

/// Regression: SchDesignator records must survive SchDoc save/load roundtrip.
#[test]
fn test_designator_survives_schdoc_roundtrip() {
    use crate::io::SchDoc;
    use crate::records::sch::designator::SchDesignator;
    use crate::records::sch::{SchRecord, SchParameter, SchLabel, SchGraphicalBase, SchPrimitiveBase};
    use std::io::Cursor;

    let mut doc = SchDoc::default();

    // Add a component
    let comp = SchComponent {
        lib_reference: "TEST_IC".to_string(),
        part_count: 1,
        display_mode_count: 1,
        current_part_id: 1,
        ..Default::default()
    };
    doc.primitives.push(SchRecord::Component(comp));

    // Add a designator as child of component (index 0)
    let label = SchLabel {
        graphical: SchGraphicalBase {
            base: SchPrimitiveBase {
                owner_index: 0,
                ..Default::default()
            },
            location_x: 100,
            location_y: 200,
            ..Default::default()
        },
        text: "U1".to_string(),
        font_id: 1,
        ..Default::default()
    };
    let param = SchParameter {
        label,
        name: "Designator".to_string(),
        read_only_state: 1,
        ..Default::default()
    };
    doc.primitives.push(SchRecord::Designator(SchDesignator {
        param,
        ..Default::default()
    }));

    // Save to buffer
    let mut buffer = Cursor::new(Vec::new());
    doc.save(&mut buffer).expect("Save must succeed");

    // Reload from buffer
    buffer.set_position(0);
    let loaded = SchDoc::open(buffer).expect("Load must succeed");

    // Find designator records
    let designators: Vec<_> = loaded
        .primitives
        .iter()
        .filter(|r| matches!(r, SchRecord::Designator(_)))
        .collect();

    assert_eq!(
        designators.len(),
        1,
        "Expected 1 Designator record after roundtrip, found {}. \
         Records might be deserializing as Parameter instead.",
        designators.len()
    );

    if let SchRecord::Designator(d) = &designators[0] {
        assert_eq!(d.text(), "U1");
        assert_eq!(d.param.label.graphical.base.owner_index, 0);
    }

    // Also verify no false-positive Parameters that are actually designators
    let parameters: Vec<_> = loaded
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::Parameter(p) = r {
                Some(p)
            } else {
                None
            }
        })
        .collect();

    let misclassified = parameters
        .iter()
        .filter(|p| p.name == "Designator")
        .count();
    assert_eq!(
        misclassified, 0,
        "Found {} Parameter records named 'Designator' — these should be SchRecord::Designator",
        misclassified
    );
}
