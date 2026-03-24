//! Core [`PcbIr`] struct — the complete intermediate representation of a PCB board.

use crate::board::IrBoardGeometry;
use crate::component::IrComponent;
use crate::component_body::IrComponentBody;
use crate::copper::FreeCopperGeometry;
use crate::handles::{
    ComponentBodyId, ComponentId, IdMap, NetId, PolygonId, RegionId, RuleId, TextId,
};
use crate::layer_stack::IrLayerStack;
use crate::net::IrNet;
use crate::polygon::IrPolygon;
use crate::region::IrRegion;
use crate::rule::IrDesignRule;
use crate::text::IrText;

/// The complete intermediate representation of a PcbDoc board.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PcbIr {
    pub board: IrBoardGeometry,
    pub layer_stack: IrLayerStack,
    pub components: IdMap<ComponentId, IrComponent>,
    pub nets: IdMap<NetId, IrNet>,
    pub rules: IdMap<RuleId, IrDesignRule>,
    pub free_copper: FreeCopperGeometry,
    pub polygons: IdMap<PolygonId, IrPolygon>,
    pub texts: IdMap<TextId, IrText>,
    pub regions: IdMap<RegionId, IrRegion>,
    pub component_bodies: IdMap<ComponentBodyId, IrComponentBody>,
}
