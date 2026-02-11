//! Leaf wrapper views and `WrapperFamily` marker types for all record types.
//!
//! Each leaf wrapper is a thin `Deref`/`DerefMut` view over a single record
//! type. The [`impl_leaf_wrapper!`] macro generates the struct, constructor,
//! and `Deref`/`DerefMut` impls.
//!
//! The wrapper holds a `&'a mut RecordNode` + a cached copy of the record.
//! On `DerefMut`, the dirty flag is set. On `Drop`, if dirty, the cached
//! record's origin is flushed back to the node.
//!
//! Each wrapper also has a corresponding `WrapperFamily` marker enum that
//! ties the record to its view for use as a type parameter in query APIs.

// ---------------------------------------------------------------------------
// impl_leaf_wrapper! macro
// ---------------------------------------------------------------------------

/// Generates a leaf view wrapper struct that caches a record from a `RecordNode`.
///
/// The wrapper:
/// - Clones the record from the node's origin on construction
/// - Provides `Deref<Target = Record>` for read access
/// - Provides `DerefMut` for write access (sets dirty flag)
/// - On `Drop`, if dirty, flushes the cached record's origin back to the node
///
/// # Usage
///
/// ```ignore
/// impl_leaf_wrapper!(SchPinView wraps SchPinRecord);
/// ```
macro_rules! impl_leaf_wrapper {
    ($view:ident wraps $record:ty) => {
        pub struct $view<'a> {
            node: &'a mut crate::v2::backing_store::RecordNode,
            cached: $record,
            dirty: bool,
        }

        impl<'a> $view<'a> {
            pub fn new(node: &'a mut crate::v2::backing_store::RecordNode) -> Self {
                let cached = <$record>::from_origin(node.origin.clone());
                Self { node, cached, dirty: false }
            }
        }

        impl<'a> std::ops::Deref for $view<'a> {
            type Target = $record;
            fn deref(&self) -> &$record {
                &self.cached
            }
        }

        impl<'a> std::ops::DerefMut for $view<'a> {
            fn deref_mut(&mut self) -> &mut $record {
                self.dirty = true;
                &mut self.cached
            }
        }

        impl<'a> Drop for $view<'a> {
            fn drop(&mut self) {
                if self.dirty {
                    use crate::v2::traits::RecordType;
                    self.node.origin = self.cached.origin().clone();
                    self.node.mark_dirty();
                }
            }
        }
    };
}

/// Generates a `WrapperFamily` marker enum and its implementation.
///
/// # Usage
///
/// ```ignore
/// impl_wrapper_family!(SchPin, SchPinRecord, SchPinView);
/// ```
macro_rules! impl_wrapper_family {
    ($marker:ident, $record:ty, $view:ident) => {
        pub enum $marker {}
        impl crate::v2::traits::WrapperFamily for $marker {
            type Record = $record;
            type View<'a> = $view<'a>;
        }
    };
}

// ---------------------------------------------------------------------------
// Schematic leaf wrappers
// ---------------------------------------------------------------------------

use crate::v2::records::{
    SchPinRecord, SchArcRecord, SchLineRecord, SchRectangleRecord, SchBezierRecord,
    SchPolylineRecord, SchPolygonRecord, SchEllipseRecord, SchPieRecord,
    SchRoundRectangleRecord, SchEllipticalArcRecord, SchImageRecord,
    SchDesignatorRecord, SchParameterRecord, SchSymbolRecord, SchLabelRecord,
    SchPowerRecord, SchPortRecord, SchNoERCRecord, SchNetLabelRecord,
    SchBusRecord, SchWireRecord, SchTextFrameRecord, SchJunctionRecord,
    SchSheetRecord, SchSheetNameRecord, SchSheetFileNameRecord,
    SchBusEntryRecord, SchSheetSymbolRecord, SchSheetEntryRecord,
    SchImplementationListRecord, SchImplementationRecord, SchNoteRecord,
    SchBlanketRecord,
};

use crate::v2::records::{
    PcbTrackRecord, PcbArcRecord, PcbFillRecord, PcbPadRecord, PcbViaRecord,
    PcbTextRecord, PcbRegionRecord, PcbComponentBodyRecord, PcbFootprintRecord,
};

// --- Schematic leaf wrappers ---

impl_leaf_wrapper!(SchPinView wraps SchPinRecord);
impl_leaf_wrapper!(SchArcView wraps SchArcRecord);
impl_leaf_wrapper!(SchLineView wraps SchLineRecord);
impl_leaf_wrapper!(SchRectangleView wraps SchRectangleRecord);
impl_leaf_wrapper!(SchBezierView wraps SchBezierRecord);
impl_leaf_wrapper!(SchPolylineView wraps SchPolylineRecord);
impl_leaf_wrapper!(SchPolygonView wraps SchPolygonRecord);
impl_leaf_wrapper!(SchEllipseView wraps SchEllipseRecord);
impl_leaf_wrapper!(SchPieView wraps SchPieRecord);
impl_leaf_wrapper!(SchRoundRectangleView wraps SchRoundRectangleRecord);
impl_leaf_wrapper!(SchEllipticalArcView wraps SchEllipticalArcRecord);
impl_leaf_wrapper!(SchImageView wraps SchImageRecord);
impl_leaf_wrapper!(SchDesignatorView wraps SchDesignatorRecord);
impl_leaf_wrapper!(SchParameterView wraps SchParameterRecord);
impl_leaf_wrapper!(SchSymbolView wraps SchSymbolRecord);
impl_leaf_wrapper!(SchLabelView wraps SchLabelRecord);
impl_leaf_wrapper!(SchPowerView wraps SchPowerRecord);
impl_leaf_wrapper!(SchPortView wraps SchPortRecord);
impl_leaf_wrapper!(SchNoERCView wraps SchNoERCRecord);
impl_leaf_wrapper!(SchNetLabelView wraps SchNetLabelRecord);
impl_leaf_wrapper!(SchBusView wraps SchBusRecord);
impl_leaf_wrapper!(SchWireView wraps SchWireRecord);
impl_leaf_wrapper!(SchTextFrameView wraps SchTextFrameRecord);
impl_leaf_wrapper!(SchJunctionView wraps SchJunctionRecord);
impl_leaf_wrapper!(SchSheetView wraps SchSheetRecord);
impl_leaf_wrapper!(SchSheetNameView wraps SchSheetNameRecord);
impl_leaf_wrapper!(SchSheetFileNameView wraps SchSheetFileNameRecord);
impl_leaf_wrapper!(SchBusEntryView wraps SchBusEntryRecord);
impl_leaf_wrapper!(SchSheetSymbolView wraps SchSheetSymbolRecord);
impl_leaf_wrapper!(SchSheetEntryView wraps SchSheetEntryRecord);
impl_leaf_wrapper!(SchImplementationListView wraps SchImplementationListRecord);
impl_leaf_wrapper!(SchImplementationView wraps SchImplementationRecord);
impl_leaf_wrapper!(SchNoteView wraps SchNoteRecord);
impl_leaf_wrapper!(SchBlanketView wraps SchBlanketRecord);

// --- PCB leaf wrappers ---

impl_leaf_wrapper!(PcbTrackView wraps PcbTrackRecord);
impl_leaf_wrapper!(PcbArcView wraps PcbArcRecord);
impl_leaf_wrapper!(PcbFillView wraps PcbFillRecord);
impl_leaf_wrapper!(PcbPadView wraps PcbPadRecord);
impl_leaf_wrapper!(PcbViaView wraps PcbViaRecord);
impl_leaf_wrapper!(PcbTextView wraps PcbTextRecord);
impl_leaf_wrapper!(PcbRegionView wraps PcbRegionRecord);
impl_leaf_wrapper!(PcbComponentBodyView wraps PcbComponentBodyRecord);
// Note: PcbFootprintRecord's leaf wrapper is named PcbFootprintMetadataView
// to avoid collision with the parent PcbFootprintView in pcb_footprint_view.rs.
impl_leaf_wrapper!(PcbFootprintMetadataView wraps PcbFootprintRecord);

// ---------------------------------------------------------------------------
// WrapperFamily marker types
// ---------------------------------------------------------------------------

// --- Schematic WrapperFamily markers ---

impl_wrapper_family!(SchPin, SchPinRecord, SchPinView);
impl_wrapper_family!(SchArc, SchArcRecord, SchArcView);
impl_wrapper_family!(SchLine, SchLineRecord, SchLineView);
impl_wrapper_family!(SchRectangle, SchRectangleRecord, SchRectangleView);
impl_wrapper_family!(SchBezier, SchBezierRecord, SchBezierView);
impl_wrapper_family!(SchPolyline, SchPolylineRecord, SchPolylineView);
impl_wrapper_family!(SchPolygon, SchPolygonRecord, SchPolygonView);
impl_wrapper_family!(SchEllipse, SchEllipseRecord, SchEllipseView);
impl_wrapper_family!(SchPie, SchPieRecord, SchPieView);
impl_wrapper_family!(SchRoundRectangle, SchRoundRectangleRecord, SchRoundRectangleView);
impl_wrapper_family!(SchEllipticalArc, SchEllipticalArcRecord, SchEllipticalArcView);
impl_wrapper_family!(SchImage, SchImageRecord, SchImageView);
impl_wrapper_family!(SchDesignator, SchDesignatorRecord, SchDesignatorView);
impl_wrapper_family!(SchParameter, SchParameterRecord, SchParameterView);
impl_wrapper_family!(SchSymbol, SchSymbolRecord, SchSymbolView);
impl_wrapper_family!(SchLabel, SchLabelRecord, SchLabelView);
impl_wrapper_family!(SchPower, SchPowerRecord, SchPowerView);
impl_wrapper_family!(SchPort, SchPortRecord, SchPortView);
impl_wrapper_family!(SchNoERC, SchNoERCRecord, SchNoERCView);
impl_wrapper_family!(SchNetLabel, SchNetLabelRecord, SchNetLabelView);
impl_wrapper_family!(SchBus, SchBusRecord, SchBusView);
impl_wrapper_family!(SchWire, SchWireRecord, SchWireView);
impl_wrapper_family!(SchTextFrame, SchTextFrameRecord, SchTextFrameView);
impl_wrapper_family!(SchJunction, SchJunctionRecord, SchJunctionView);
impl_wrapper_family!(SchSheet, SchSheetRecord, SchSheetView);
impl_wrapper_family!(SchSheetName, SchSheetNameRecord, SchSheetNameView);
impl_wrapper_family!(SchSheetFileName, SchSheetFileNameRecord, SchSheetFileNameView);
impl_wrapper_family!(SchBusEntry, SchBusEntryRecord, SchBusEntryView);
impl_wrapper_family!(SchSheetSymbol, SchSheetSymbolRecord, SchSheetSymbolView);
impl_wrapper_family!(SchSheetEntry, SchSheetEntryRecord, SchSheetEntryView);
impl_wrapper_family!(SchImplementationList, SchImplementationListRecord, SchImplementationListView);
impl_wrapper_family!(SchImplementation, SchImplementationRecord, SchImplementationView);
impl_wrapper_family!(SchNote, SchNoteRecord, SchNoteView);
impl_wrapper_family!(SchBlanket, SchBlanketRecord, SchBlanketView);

// --- PCB WrapperFamily markers ---

impl_wrapper_family!(PcbTrack, PcbTrackRecord, PcbTrackView);
impl_wrapper_family!(PcbArc, PcbArcRecord, PcbArcView);
impl_wrapper_family!(PcbFill, PcbFillRecord, PcbFillView);
impl_wrapper_family!(PcbPad, PcbPadRecord, PcbPadView);
impl_wrapper_family!(PcbVia, PcbViaRecord, PcbViaView);
impl_wrapper_family!(PcbText, PcbTextRecord, PcbTextView);
impl_wrapper_family!(PcbRegion, PcbRegionRecord, PcbRegionView);
impl_wrapper_family!(PcbComponentBody, PcbComponentBodyRecord, PcbComponentBodyView);
impl_wrapper_family!(PcbFootprintMetadata, PcbFootprintRecord, PcbFootprintMetadataView);
