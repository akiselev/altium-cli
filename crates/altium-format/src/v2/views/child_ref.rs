//! Opaque child reference types for read-only browsing.
//!
//! These wrap `&RecordNode` without exposing the backing store internals,
//! allowing external code to inspect record types and clone typed records.

use crate::v2::backing_store::RecordNode;
use crate::v2::records::*;
use crate::v2::traits::RecordType;

// ---------------------------------------------------------------------------
// SchChildRef — opaque ref to a schematic child record
// ---------------------------------------------------------------------------

/// Opaque read-only reference to a schematic child record.
///
/// Hides `RecordNode` from external consumers while exposing the record
/// type ID and typed access via `as_*` methods. Each `as_*` method clones
/// the underlying origin, so is best used sparingly.
pub struct SchChildRef<'a> {
    node: &'a RecordNode,
}

impl<'a> SchChildRef<'a> {
    pub(crate) fn new(node: &'a RecordNode) -> Self {
        Self { node }
    }

    /// The numeric record type ID (RECORD value).
    pub fn record_id(&self) -> u8 {
        self.node.key
    }

    pub fn as_pin(&self) -> Option<SchPinRecord> {
        (self.node.key == SchPinRecord::RECORD_ID)
            .then(|| SchPinRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_arc(&self) -> Option<SchArcRecord> {
        (self.node.key == SchArcRecord::RECORD_ID)
            .then(|| SchArcRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_line(&self) -> Option<SchLineRecord> {
        (self.node.key == SchLineRecord::RECORD_ID)
            .then(|| SchLineRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_rectangle(&self) -> Option<SchRectangleRecord> {
        (self.node.key == SchRectangleRecord::RECORD_ID)
            .then(|| SchRectangleRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_bezier(&self) -> Option<SchBezierRecord> {
        (self.node.key == SchBezierRecord::RECORD_ID)
            .then(|| SchBezierRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_polyline(&self) -> Option<SchPolylineRecord> {
        (self.node.key == SchPolylineRecord::RECORD_ID)
            .then(|| SchPolylineRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_polygon(&self) -> Option<SchPolygonRecord> {
        (self.node.key == SchPolygonRecord::RECORD_ID)
            .then(|| SchPolygonRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_ellipse(&self) -> Option<SchEllipseRecord> {
        (self.node.key == SchEllipseRecord::RECORD_ID)
            .then(|| SchEllipseRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_pie(&self) -> Option<SchPieRecord> {
        (self.node.key == SchPieRecord::RECORD_ID)
            .then(|| SchPieRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_round_rectangle(&self) -> Option<SchRoundRectangleRecord> {
        (self.node.key == SchRoundRectangleRecord::RECORD_ID)
            .then(|| SchRoundRectangleRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_elliptical_arc(&self) -> Option<SchEllipticalArcRecord> {
        (self.node.key == SchEllipticalArcRecord::RECORD_ID)
            .then(|| SchEllipticalArcRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_image(&self) -> Option<SchImageRecord> {
        (self.node.key == SchImageRecord::RECORD_ID)
            .then(|| SchImageRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_designator(&self) -> Option<SchDesignatorRecord> {
        (self.node.key == SchDesignatorRecord::RECORD_ID)
            .then(|| SchDesignatorRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_parameter(&self) -> Option<SchParameterRecord> {
        (self.node.key == SchParameterRecord::RECORD_ID)
            .then(|| SchParameterRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_symbol(&self) -> Option<SchSymbolRecord> {
        (self.node.key == SchSymbolRecord::RECORD_ID)
            .then(|| SchSymbolRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_label(&self) -> Option<SchLabelRecord> {
        (self.node.key == SchLabelRecord::RECORD_ID)
            .then(|| SchLabelRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_implementation_list(&self) -> Option<SchImplementationListRecord> {
        (self.node.key == SchImplementationListRecord::RECORD_ID)
            .then(|| SchImplementationListRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_implementation(&self) -> Option<SchImplementationRecord> {
        (self.node.key == SchImplementationRecord::RECORD_ID)
            .then(|| SchImplementationRecord::from_origin(self.node.origin.clone()))
    }
}

// ---------------------------------------------------------------------------
// PcbChildRef — opaque ref to a PCB primitive record
// ---------------------------------------------------------------------------

/// Opaque read-only reference to a PCB primitive record within a footprint.
///
/// Hides `RecordNode` from external consumers while exposing the primitive
/// type ID and typed access via `as_*` methods.
pub struct PcbChildRef<'a> {
    node: &'a RecordNode,
}

impl<'a> PcbChildRef<'a> {
    pub(crate) fn new(node: &'a RecordNode) -> Self {
        Self { node }
    }

    /// The numeric primitive type ID (from `PcbObjectId`).
    pub fn type_id(&self) -> u8 {
        self.node.key
    }

    pub fn as_pad(&self) -> Option<PcbPadRecord> {
        (self.node.key == PcbPadRecord::RECORD_ID)
            .then(|| PcbPadRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_track(&self) -> Option<PcbTrackRecord> {
        (self.node.key == PcbTrackRecord::RECORD_ID)
            .then(|| PcbTrackRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_arc(&self) -> Option<PcbArcRecord> {
        (self.node.key == PcbArcRecord::RECORD_ID)
            .then(|| PcbArcRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_via(&self) -> Option<PcbViaRecord> {
        (self.node.key == PcbViaRecord::RECORD_ID)
            .then(|| PcbViaRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_fill(&self) -> Option<PcbFillRecord> {
        (self.node.key == PcbFillRecord::RECORD_ID)
            .then(|| PcbFillRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_text(&self) -> Option<PcbTextRecord> {
        (self.node.key == PcbTextRecord::RECORD_ID)
            .then(|| PcbTextRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_region(&self) -> Option<PcbRegionRecord> {
        (self.node.key == PcbRegionRecord::RECORD_ID)
            .then(|| PcbRegionRecord::from_origin(self.node.origin.clone()))
    }

    pub fn as_component_body(&self) -> Option<PcbComponentBodyRecord> {
        (self.node.key == PcbComponentBodyRecord::RECORD_ID)
            .then(|| PcbComponentBodyRecord::from_origin(self.node.origin.clone()))
    }
}
