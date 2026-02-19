//! Builder types for constructing components and footprints from templates.
//!
//! Builders use the template+setter pattern: create a record from a template
//! function, then configure it via typed `&mut Record` closures. External code
//! never sees `RecordOrigin` or `RecordNode`.

use crate::v2::backing_store::{
    PcbPrimitiveRef, RecordNode, RecordOrigin,
};
use crate::v2::records::{
    SchComponentRecord, SchPinRecord, SchArcRecord, SchLineRecord, SchRectangleRecord,
    SchLabelRecord, SchDesignatorRecord, SchParameterRecord, SchSymbolRecord,
    PcbFootprintRecord, PcbPadRecord, PcbTrackRecord, PcbArcRecord,
};
use crate::v2::traits::RecordType;

// ---------------------------------------------------------------------------
// ComponentBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing schematic components from templates.
///
/// Closures receive typed `&mut Record` references, not raw `RecordOrigin`.
///
/// # Example
///
/// ```ignore
/// let mut builder = ComponentBuilder::new(templates::sch_component_default);
/// builder.with_component(|comp| {
///     comp.set_lib_reference(LibReference::from("LM358"));
///     comp.set_description(Description::from("Dual Op-Amp"));
/// });
/// builder.add_pin(templates::sch_pin_default, |pin| {
///     pin.set_designator(Designator::from("1"));
///     pin.set_name(PinName::from("VCC"));
/// });
/// let group = builder.build();
/// ```
pub struct ComponentBuilder {
    component: RecordNode,
    children: Vec<RecordNode>,
}

impl ComponentBuilder {
    /// Create a new builder using the given template for the component record.
    pub fn new(template: fn() -> RecordOrigin) -> Self {
        let origin = template();
        let key = match &origin {
            RecordOrigin::Param(p) => {
                p.params
                    .get("RECORD")
                    .map(|v| v.as_int_or(0) as u8)
                    .unwrap_or(1)
            }
            _ => 1,
        };
        Self {
            component: RecordNode::new(key, origin),
            children: Vec::new(),
        }
    }

    /// Modify the component record via a typed closure.
    pub fn with_component(
        &mut self,
        f: impl FnOnce(&mut SchComponentRecord),
    ) -> &mut Self {
        let mut record = SchComponentRecord::from_origin(self.component.origin.clone());
        f(&mut record);
        self.component.origin = record.origin().clone();
        self.component.mark_dirty();
        self
    }

    /// Add a pin using the given template, configured via a typed closure.
    pub fn add_pin(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut SchPinRecord),
    ) -> &mut Self {
        let mut record = SchPinRecord::from_origin(template());
        build(&mut record);
        let mut node = RecordNode::new(SchPinRecord::RECORD_ID, record.origin().clone());
        node.mark_dirty();
        self.children.push(node);
        self
    }

    /// Add an arc child using the given template.
    pub fn add_arc(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut SchArcRecord),
    ) -> &mut Self {
        let mut record = SchArcRecord::from_origin(template());
        build(&mut record);
        let mut node = RecordNode::new(SchArcRecord::RECORD_ID, record.origin().clone());
        node.mark_dirty();
        self.children.push(node);
        self
    }

    /// Add a line child using the given template.
    pub fn add_line(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut SchLineRecord),
    ) -> &mut Self {
        let mut record = SchLineRecord::from_origin(template());
        build(&mut record);
        let mut node = RecordNode::new(SchLineRecord::RECORD_ID, record.origin().clone());
        node.mark_dirty();
        self.children.push(node);
        self
    }

    /// Add a rectangle child using the given template.
    pub fn add_rectangle(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut SchRectangleRecord),
    ) -> &mut Self {
        let mut record = SchRectangleRecord::from_origin(template());
        build(&mut record);
        let mut node = RecordNode::new(SchRectangleRecord::RECORD_ID, record.origin().clone());
        node.mark_dirty();
        self.children.push(node);
        self
    }

    /// Add a label child using the given template.
    pub fn add_label(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut SchLabelRecord),
    ) -> &mut Self {
        let mut record = SchLabelRecord::from_origin(template());
        build(&mut record);
        let mut node = RecordNode::new(SchLabelRecord::RECORD_ID, record.origin().clone());
        node.mark_dirty();
        self.children.push(node);
        self
    }

    /// Add a designator child using the given template.
    pub fn add_designator(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut SchDesignatorRecord),
    ) -> &mut Self {
        let mut record = SchDesignatorRecord::from_origin(template());
        build(&mut record);
        let mut node = RecordNode::new(SchDesignatorRecord::RECORD_ID, record.origin().clone());
        node.mark_dirty();
        self.children.push(node);
        self
    }

    /// Add a parameter child using the given template.
    pub fn add_parameter(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut SchParameterRecord),
    ) -> &mut Self {
        let mut record = SchParameterRecord::from_origin(template());
        build(&mut record);
        let mut node = RecordNode::new(SchParameterRecord::RECORD_ID, record.origin().clone());
        node.mark_dirty();
        self.children.push(node);
        self
    }

    /// Add a symbol child using the given template.
    pub fn add_symbol(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut SchSymbolRecord),
    ) -> &mut Self {
        let mut record = SchSymbolRecord::from_origin(template());
        build(&mut record);
        let mut node = RecordNode::new(SchSymbolRecord::RECORD_ID, record.origin().clone());
        node.mark_dirty();
        self.children.push(node);
        self
    }

    /// Consume the builder, returning (component_node, children_nodes).
    pub(crate) fn build(self) -> (RecordNode, Vec<RecordNode>) {
        (self.component, self.children)
    }
}

// ---------------------------------------------------------------------------
// FootprintBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing PCB footprints from templates.
///
/// Closures receive typed `&mut Record` references, not raw `RecordOrigin`.
///
/// # Example
///
/// ```ignore
/// let mut builder = FootprintBuilder::new(templates::pcb_footprint_default);
/// builder.with_metadata(|fp| {
///     fp.set_pattern("SOIC-8".into());
///     fp.set_description("8-pin SOIC".into());
/// });
/// builder.add_pad(templates::pcb_pad_default, |pad| {
///     pad.set_position_x(PcbCoord::from_mm(1.27));
///     pad.set_top_size_x(PcbCoord::from_mm(0.6));
/// });
/// let group = builder.build();
/// ```
pub struct FootprintBuilder {
    metadata: RecordNode,
    primitives: Vec<RecordNode>,
    primitive_refs: Vec<PcbPrimitiveRef>,
}

impl FootprintBuilder {
    /// Create a new builder using the given template for the footprint metadata.
    pub fn new(template: fn() -> RecordOrigin) -> Self {
        Self {
            metadata: RecordNode::new(0, template()),
            primitives: Vec::new(),
            primitive_refs: Vec::new(),
        }
    }

    /// Modify the footprint metadata via a typed closure.
    pub fn with_metadata(
        &mut self,
        f: impl FnOnce(&mut PcbFootprintRecord),
    ) -> &mut Self {
        let mut record = PcbFootprintRecord::from_origin(self.metadata.origin.clone());
        f(&mut record);
        self.metadata.origin = record.origin().clone();
        self.metadata.mark_dirty();
        self
    }

    /// Add a pad primitive using the given template, configured via a typed closure.
    pub fn add_pad(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut PcbPadRecord),
    ) -> &mut Self {
        let mut record = PcbPadRecord::from_origin(template());
        build(&mut record);
        let key = PcbPadRecord::RECORD_ID;
        let mut node = RecordNode::new(key, record.origin().clone());
        node.mark_dirty();
        let index = self.primitives.len();
        self.primitives.push(node);
        self.primitive_refs.push(PcbPrimitiveRef::new(key, index));
        self
    }

    /// Add a track primitive using the given template, configured via a typed closure.
    pub fn add_track(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut PcbTrackRecord),
    ) -> &mut Self {
        let mut record = PcbTrackRecord::from_origin(template());
        build(&mut record);
        let key = PcbTrackRecord::RECORD_ID;
        let mut node = RecordNode::new(key, record.origin().clone());
        node.mark_dirty();
        let index = self.primitives.len();
        self.primitives.push(node);
        self.primitive_refs.push(PcbPrimitiveRef::new(key, index));
        self
    }

    /// Add an arc primitive using the given template, configured via a typed closure.
    pub fn add_arc(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut PcbArcRecord),
    ) -> &mut Self {
        let mut record = PcbArcRecord::from_origin(template());
        build(&mut record);
        let key = PcbArcRecord::RECORD_ID;
        let mut node = RecordNode::new(key, record.origin().clone());
        node.mark_dirty();
        let index = self.primitives.len();
        self.primitives.push(node);
        self.primitive_refs.push(PcbPrimitiveRef::new(key, index));
        self
    }

    /// Consume the builder, returning (metadata_node, primitive_nodes, primitive_refs).
    pub(crate) fn build(self) -> (RecordNode, Vec<RecordNode>, Vec<PcbPrimitiveRef>) {
        (self.metadata, self.primitives, self.primitive_refs)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::coord::{AltiumCoord, PcbCoord};
    use crate::v2::newtypes::{Designator, LibReference, PinName};
    use crate::v2::records::enums::PinElectricalType;
    use crate::v2::records::SchPinRecord;
    use crate::v2::templates;

    #[test]
    fn component_builder_basic() {
        let (comp, children) = ComponentBuilder::new(templates::sch_component_default).build();

        assert_eq!(comp.key, 1);
        assert!(children.is_empty());
    }

    #[test]
    fn component_builder_with_pins() {
        let mut builder = ComponentBuilder::new(templates::sch_component_default);
        builder.add_pin(templates::sch_pin_default, |pin| {
            pin.set_designator(Designator::from("1"));
            pin.set_name(PinName::from("VCC"));
        });
        builder.add_pin(templates::sch_pin_default, |pin| {
            pin.set_designator(Designator::from("2"));
            pin.set_name(PinName::from("GND"));
        });
        let (comp, children) = builder.build();

        assert_eq!(comp.key, 1);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].key, 2); // pin record_id
        assert_eq!(children[1].key, 2);

        // Verify the pin data was actually written through the typed closure
        let pin0 = SchPinRecord::from_origin(children[0].origin.clone());
        assert_eq!(pin0.name(), PinName::from("VCC"));
        assert_eq!(pin0.designator(), Designator::from("1"));

        let pin1 = SchPinRecord::from_origin(children[1].origin.clone());
        assert_eq!(pin1.name(), PinName::from("GND"));
        assert_eq!(pin1.designator(), Designator::from("2"));
    }

    #[test]
    fn component_builder_with_component_modification() {
        let mut builder = ComponentBuilder::new(templates::sch_component_default);
        builder.with_component(|comp| {
            comp.set_lib_reference(LibReference::from("LM358"));
        });
        let (comp, _children) = builder.build();

        let record = SchComponentRecord::from_origin(comp.origin.clone());
        assert_eq!(record.lib_reference(), LibReference::from("LM358"));
    }

    #[test]
    fn footprint_builder_basic() {
        let (metadata, primitives, _refs) = FootprintBuilder::new(templates::pcb_footprint_default).build();

        assert_eq!(metadata.key, 0);
        assert!(primitives.is_empty());
    }

    #[test]
    fn footprint_builder_with_primitives() {
        let mut builder = FootprintBuilder::new(templates::pcb_footprint_default);
        builder.add_pad(templates::pcb_pad_default, |pad| {
            pad.set_position_x(PcbCoord::from_raw(100_000));
        });
        builder.add_track(templates::pcb_track_default, |track| {
            track.set_width(PcbCoord::from_raw(10_000));
        });
        builder.add_track(templates::pcb_track_default, |_| {});
        let (_, primitives, prim_refs) = builder.build();

        assert_eq!(primitives.len(), 3);
        assert_eq!(primitives[0].key, 2); // pad type_id
        assert_eq!(primitives[1].key, 4); // track type_id
        assert_eq!(primitives[2].key, 4);
        assert_eq!(prim_refs.len(), 3);

        // Verify the pad data was actually written
        let pad = PcbPadRecord::from_origin(primitives[0].origin.clone());
        assert_eq!(pad.position_x().to_raw(), 100_000);

        // Verify the track data was actually written
        let track = PcbTrackRecord::from_origin(primitives[1].origin.clone());
        assert_eq!(track.width().to_raw(), 10_000);
    }

    #[test]
    fn footprint_builder_with_metadata() {
        let mut builder = FootprintBuilder::new(templates::pcb_footprint_default);
        builder.with_metadata(|fp| {
            fp.set_pattern("SOIC-8".to_string());
        });
        let (metadata, _, _) = builder.build();

        let record = PcbFootprintRecord::from_origin(metadata.origin.clone());
        assert_eq!(record.pattern(), "SOIC-8");
    }

    #[test]
    fn template_sch_pin_has_record_id() {
        let origin = templates::sch_pin_default();
        let params = origin.param().params.get("RECORD").unwrap();
        assert_eq!(params.as_int_or(0), 2);
    }

    #[test]
    fn template_sch_component_has_record_id() {
        let origin = templates::sch_component_default();
        let params = origin.param().params.get("RECORD").unwrap();
        assert_eq!(params.as_int_or(0), 1);
    }

    #[test]
    fn template_sch_pin_roundtrip() {
        let origin = templates::sch_pin_default();
        let record = SchPinRecord::from_origin(origin);
        assert_eq!(record.electrical(), PinElectricalType::Passive);
        assert_eq!(&*record.name(), "");
        assert_eq!(&*record.designator(), "");
    }

    #[test]
    fn template_sch_rectangle_has_defaults() {
        let origin = templates::sch_rectangle_default();
        let p = origin.param();
        assert_eq!(p.params.get("RECORD").unwrap().as_int_or(0), 14);
        assert!(p.params.get("ISSOLID").unwrap().as_bool_or(false));
    }

    #[test]
    fn template_pcb_footprint_has_pattern() {
        let origin = templates::pcb_footprint_default();
        let p = origin.param();
        assert!(p.params.contains("PATTERN"));
    }

    #[test]
    fn template_pcb_pad_has_field_spans() {
        let origin = templates::pcb_pad_default();
        let record = PcbPadRecord::from_origin(origin);
        // Default pad should have MultiLayer (74)
        assert_eq!(record.layer(), 74);
        // Default values should be 0
        assert_eq!(record.position_x().to_raw(), 0);
        assert_eq!(record.position_y().to_raw(), 0);
        assert_eq!(record.hole_size().to_raw(), 0);
    }

    #[test]
    fn template_pcb_pad_setters_work() {
        let origin = templates::pcb_pad_default();
        let mut record = PcbPadRecord::from_origin(origin);

        record.set_position_x(PcbCoord::from_raw(100_000));
        record.set_top_size_x(PcbCoord::from_raw(50_000));
        record.set_top_size_y(PcbCoord::from_raw(60_000));
        record.set_hole_size(PcbCoord::from_raw(10_000));
        record.set_top_shape(1);
        record.set_is_plated(true);

        assert_eq!(record.position_x().to_raw(), 100_000);
        assert_eq!(record.top_size_x().to_raw(), 50_000);
        assert_eq!(record.top_size_y().to_raw(), 60_000);
        assert_eq!(record.hole_size().to_raw(), 10_000);
        assert_eq!(record.top_shape(), 1);
        assert!(record.is_plated());
    }

    #[test]
    fn builder_end_to_end_component_with_children() {
        // Full end-to-end: build a component with pins, verify everything
        let mut builder = ComponentBuilder::new(templates::sch_component_default);
        builder.with_component(|comp| {
            comp.set_lib_reference(LibReference::from("Resistor"));
        });
        builder.add_pin(templates::sch_pin_default, |pin| {
            pin.set_designator(Designator::from("1"));
            pin.set_name(PinName::from("A"));
            pin.set_electrical(PinElectricalType::Passive);
        });
        builder.add_pin(templates::sch_pin_default, |pin| {
            pin.set_designator(Designator::from("2"));
            pin.set_name(PinName::from("B"));
            pin.set_electrical(PinElectricalType::Passive);
        });
        let (component, children) = builder.build();

        // Verify component
        let comp = SchComponentRecord::from_origin(component.origin.clone());
        assert_eq!(comp.lib_reference(), LibReference::from("Resistor"));

        // Verify children
        assert_eq!(children.len(), 2);
        let pin0 = SchPinRecord::from_origin(children[0].origin.clone());
        assert_eq!(pin0.designator(), Designator::from("1"));
        assert_eq!(pin0.electrical(), PinElectricalType::Passive);

        let pin1 = SchPinRecord::from_origin(children[1].origin.clone());
        assert_eq!(pin1.designator(), Designator::from("2"));
    }
}
