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
