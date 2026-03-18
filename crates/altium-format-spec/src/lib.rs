pub mod ast;
pub mod compiler;
pub mod diagnostic;
pub mod dump;
pub mod eco;
pub mod eval;
pub mod executor;
pub mod formatter;
pub mod import;
pub mod lexer;
pub mod model;
pub mod parser;
pub mod reconciler;
pub mod trivia;

// Public API re-exports
pub use compiler::{compile_spec, compile_spec_with_imports, compile_spec_with_resolved, compile_imported_schlibs};
pub use dump::{dump_intlib, dump_pcbdoc, dump_pcblib, dump_prjpcb, dump_schdoc, dump_schlib, dump_placement_block, IntLibDump};
pub use executor::{apply_spec_pcbdoc, apply_spec_pcblib, apply_spec_prjpcb, apply_spec_schdoc, apply_spec_schlib};
pub use import::{ResolvedSpec, resolve_imports};
pub use reconciler::{reconcile_pcbdoc, reconcile_pcbdoc_empty, reconcile_pcblib, reconcile_pcblib_empty, reconcile_prjpcb, reconcile_prjpcb_empty, reconcile_schdoc, reconcile_schdoc_empty, reconcile_schlib, reconcile_schlib_empty};
pub use eval::{
    EvalResult, ScopeStack, SpecError, SpecErrorCode, Value, eval_expr, eval_let_bindings,
    unit_to_internal,
};
pub use eco::{
    EngineeringChangeOrder, EntityChange, EntityKind, EcoSummary, KindSummary,
    PropChange, PropValue,
};
pub use formatter::{format_spec, FormatConfig, FormatResult};
pub use trivia::{CommentToken, TriviaMap, parse_with_trivia};
pub use model::{
    AutoplaceConfig, BoardSpec, ComponentSpec, FootprintMapSpec, FootprintSpec, GraphicProperties,
    GraphicSpec, GraphicType, PadSpec, ParameterSpec, PartSpec, PcbDocSpec, PcbGraphicProperties,
    PcbGraphicSpec, PcbGraphicType, PcbLibSpec, PinPadMap, PinSpec, PlacementClearanceSpec,
    PlacementConstraintSpec, PlacementGroupSpec, PlacementOptimizeSpec, PlacementPlaceSpec,
    PlacementRuleSpec, PlacementSpec, PrjPcbSpec, ProjectSpec, SchDocSpec, SchLibSpec, SpecDomain,
    SpecModel, UnplacedStrategy,
};
