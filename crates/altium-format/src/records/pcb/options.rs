//! PCB options and settings records.
//!
//! These records store various configuration options for PCB design tools
//! such as the placer, DRC checker, and pin swap settings.

use crate::types::{Coord, ParameterCollection};

/// Advanced placer options for component placement.
#[derive(Debug, Clone, Default)]
pub struct PcbAdvancedPlacerOptions {
    /// Large component clearance.
    pub large_clearance: Coord,
    /// Small component clearance.
    pub small_clearance: Coord,
    /// Whether to use rotation during placement.
    pub use_rotation: bool,
    /// Whether to use layer swap during placement.
    pub use_layer_swap: bool,
    /// First bypass net name.
    pub bypass_net1: String,
    /// Second bypass net name.
    pub bypass_net2: String,
    /// Whether to use advanced placement algorithms.
    pub use_advanced_place: bool,
    /// Whether to use grouping during placement.
    pub use_grouping: bool,
    /// All parameters for round-tripping.
    pub params: ParameterCollection,
}

impl PcbAdvancedPlacerOptions {
    /// Parse from parameters.
    pub fn from_params(params: &ParameterCollection) -> Self {
        Self {
            large_clearance: params
                .get("PLACELARGECLEAR")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            small_clearance: params
                .get("PLACESMALLCLEAR")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            use_rotation: params
                .get("PLACEUSEROTATION")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            use_layer_swap: params
                .get("PLACEUSELAYERSWAP")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            bypass_net1: params
                .get("PLACEBYPASSNET1")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            bypass_net2: params
                .get("PLACEBYPASSNET2")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            use_advanced_place: params
                .get("PLACEUSEADVANCEDPLACE")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            use_grouping: params
                .get("PLACEUSEGROUPING")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            params: params.clone(),
        }
    }

    /// Export to parameters.
    pub fn to_params(&self) -> ParameterCollection {
        let mut params = self.params.clone();
        params.add("RECORD", "AdvancedPlacerOptions");
        params.add_coord("PLACELARGECLEAR", self.large_clearance);
        params.add_coord("PLACESMALLCLEAR", self.small_clearance);
        params.add(
            "PLACEUSEROTATION",
            if self.use_rotation { "TRUE" } else { "FALSE" },
        );
        params.add(
            "PLACEUSELAYERSWAP",
            if self.use_layer_swap { "TRUE" } else { "FALSE" },
        );
        params.add("PLACEBYPASSNET1", &self.bypass_net1);
        params.add("PLACEBYPASSNET2", &self.bypass_net2);
        params.add(
            "PLACEUSEADVANCEDPLACE",
            if self.use_advanced_place {
                "TRUE"
            } else {
                "FALSE"
            },
        );
        params.add(
            "PLACEUSEGROUPING",
            if self.use_grouping { "TRUE" } else { "FALSE" },
        );
        params
    }
}

/// Design Rule Checker options.
#[derive(Debug, Clone, Default)]
pub struct PcbDrcOptions {
    /// Whether to generate DRC file.
    pub make_drc_file: bool,
    /// Whether to generate DRC error list.
    pub make_drc_error_list: bool,
    /// Whether to show subnet details.
    pub subnet_details: bool,
    /// Report filename.
    pub report_filename: String,
    /// All parameters for round-tripping.
    pub params: ParameterCollection,
}

impl PcbDrcOptions {
    /// Parse from parameters.
    pub fn from_params(params: &ParameterCollection) -> Self {
        Self {
            make_drc_file: params
                .get("DOMAKEDRCFILE")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            make_drc_error_list: params
                .get("DOMAKEDRCERRORLIST")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            subnet_details: params
                .get("DOSUBNETDETAILS")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            report_filename: params
                .get("REPORTFILENAME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            params: params.clone(),
        }
    }

    /// Export to parameters.
    pub fn to_params(&self) -> ParameterCollection {
        let mut params = self.params.clone();
        params.add("RECORD", "DesignRuleCheckerOptions");
        params.add(
            "DOMAKEDRCFILE",
            if self.make_drc_file { "TRUE" } else { "FALSE" },
        );
        params.add(
            "DOMAKEDRCERRORLIST",
            if self.make_drc_error_list {
                "TRUE"
            } else {
                "FALSE"
            },
        );
        params.add(
            "DOSUBNETDETAILS",
            if self.subnet_details { "TRUE" } else { "FALSE" },
        );
        params.add("REPORTFILENAME", &self.report_filename);
        params
    }
}

/// Pin Swap options.
#[derive(Debug, Clone, Default)]
pub struct PcbPinSwapOptions {
    /// Whether pin swap is quiet (no prompts).
    pub quiet: bool,
    /// Whether to approximate (use closest match).
    pub approximate: bool,
    /// All parameters for round-tripping.
    pub params: ParameterCollection,
}

impl PcbPinSwapOptions {
    /// Parse from parameters.
    pub fn from_params(params: &ParameterCollection) -> Self {
        Self {
            quiet: params
                .get("QUIET")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            approximate: params
                .get("APPROXIMATE")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            params: params.clone(),
        }
    }

    /// Export to parameters.
    pub fn to_params(&self) -> ParameterCollection {
        let mut params = self.params.clone();
        params.add("RECORD", "PinSwapOptions");
        params.add("QUIET", if self.quiet { "TRUE" } else { "FALSE" });
        params.add(
            "APPROXIMATE",
            if self.approximate { "TRUE" } else { "FALSE" },
        );
        params
    }
}
