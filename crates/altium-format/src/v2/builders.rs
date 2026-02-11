//! Builder types for constructing components and footprints from templates.

use crate::v2::backing_store::{
    ComponentGroup, FootprintGroup, PcbPrimitiveRef, RecordNode, RecordOrigin,
};

// ---------------------------------------------------------------------------
// ComponentBuilder (Track 6C)
// ---------------------------------------------------------------------------

/// Builder for constructing schematic components from templates.
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

    /// Modify the component record via a closure.
    pub fn with_component<F: FnOnce(&mut RecordOrigin)>(&mut self, f: F) -> &mut Self {
        f(&mut self.component.origin);
        self.component.mark_dirty();
        self
    }

    /// Add a child record from a template, configuring it via a closure.
    pub fn add_child(
        &mut self,
        template: fn() -> RecordOrigin,
        configure: impl FnOnce(&mut RecordOrigin),
    ) -> &mut Self {
        let mut origin = template();
        configure(&mut origin);
        let key = match &origin {
            RecordOrigin::Param(p) => {
                p.params
                    .get("RECORD")
                    .map(|v| v.as_int_or(0) as u8)
                    .unwrap_or(0)
            }
            _ => 0,
        };
        let mut node = RecordNode::new(key, origin);
        node.mark_dirty();
        self.children.push(node);
        self
    }

    /// Add a pin using the given template.
    pub fn add_pin(
        &mut self,
        template: fn() -> RecordOrigin,
        configure: impl FnOnce(&mut RecordOrigin),
    ) -> &mut Self {
        self.add_child(template, configure)
    }

    /// Consume the builder into a ComponentGroup.
    pub fn build(self) -> ComponentGroup {
        ComponentGroup::new(self.component, self.children, Vec::new())
    }
}

// ---------------------------------------------------------------------------
// FootprintBuilder (Track 6C)
// ---------------------------------------------------------------------------

/// Builder for constructing PCB footprints from templates.
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

    /// Modify the footprint metadata via a closure.
    pub fn with_metadata<F: FnOnce(&mut RecordOrigin)>(&mut self, f: F) -> &mut Self {
        f(&mut self.metadata.origin);
        self.metadata.mark_dirty();
        self
    }

    /// Add a primitive record from a template.
    pub fn add_primitive(
        &mut self,
        type_id: u8,
        template: fn() -> RecordOrigin,
        configure: impl FnOnce(&mut RecordOrigin),
    ) -> &mut Self {
        let mut origin = template();
        configure(&mut origin);
        let mut node = RecordNode::new(type_id, origin);
        node.mark_dirty();
        let index = self.primitives.len();
        self.primitives.push(node);
        self.primitive_refs.push(PcbPrimitiveRef::new(type_id, index));
        self
    }

    /// Add a pad primitive.
    pub fn add_pad(
        &mut self,
        template: fn() -> RecordOrigin,
        configure: impl FnOnce(&mut RecordOrigin),
    ) -> &mut Self {
        self.add_primitive(2, template, configure)
    }

    /// Add a track primitive.
    pub fn add_track(
        &mut self,
        template: fn() -> RecordOrigin,
        configure: impl FnOnce(&mut RecordOrigin),
    ) -> &mut Self {
        self.add_primitive(4, template, configure)
    }

    /// Add an arc primitive.
    pub fn add_arc(
        &mut self,
        template: fn() -> RecordOrigin,
        configure: impl FnOnce(&mut RecordOrigin),
    ) -> &mut Self {
        self.add_primitive(1, template, configure)
    }

    /// Consume the builder into a FootprintGroup.
    pub fn build(self) -> FootprintGroup {
        FootprintGroup::new(
            self.metadata,
            self.primitives,
            Vec::new(), // raw_pattern_name_block
            self.primitive_refs,
            Vec::new(), // raw_header
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::newtypes::LibReference;
    use crate::v2::records::enums::PinElectricalType;
    use crate::v2::records::SchComponentRecord;
    use crate::v2::records::SchPinRecord;
    use crate::v2::templates;

    #[test]
    fn component_builder_basic() {
        let group = ComponentBuilder::new(templates::sch_component_default).build();

        assert_eq!(group.component.key, 1);
        assert!(group.children.is_empty());
    }

    #[test]
    fn component_builder_with_pins() {
        let mut builder = ComponentBuilder::new(templates::sch_component_default);
        builder.add_pin(templates::sch_pin_default, |_origin| {});
        builder.add_pin(templates::sch_pin_default, |_origin| {});
        let group = builder.build();

        assert_eq!(group.component.key, 1);
        assert_eq!(group.children.len(), 2);
        assert_eq!(group.children[0].key, 2); // pin record_id
        assert_eq!(group.children[1].key, 2);
    }

    #[test]
    fn component_builder_with_component_modification() {
        let mut builder = ComponentBuilder::new(templates::sch_component_default);
        builder.with_component(|origin| {
            origin.param_mut().params.add("LIBREFERENCE", "LM358");
        });
        let group = builder.build();

        let record = SchComponentRecord::from_origin(group.component.origin.clone());
        assert_eq!(record.lib_reference(), LibReference::from("LM358"));
    }

    #[test]
    fn footprint_builder_basic() {
        let group = FootprintBuilder::new(templates::pcb_footprint_default).build();

        assert_eq!(group.metadata.key, 0);
        assert!(group.primitives.is_empty());
    }

    #[test]
    fn footprint_builder_with_primitives() {
        let mut builder = FootprintBuilder::new(templates::pcb_footprint_default);
        builder.add_pad(templates::pcb_pad_default, |_| {});
        builder.add_track(templates::pcb_track_default, |_| {});
        builder.add_track(templates::pcb_track_default, |_| {});
        let group = builder.build();

        assert_eq!(group.primitives.len(), 3);
        assert_eq!(group.primitives[0].key, 2); // pad type_id
        assert_eq!(group.primitives[1].key, 4); // track type_id
        assert_eq!(group.primitives[2].key, 4);
        assert_eq!(group.original_primitive_order.len(), 3);
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
}
