pub mod ast;
pub mod compiler;
pub mod diagnostic;
pub mod dump;
pub mod eco;
pub mod eval;
pub mod executor;
pub mod import;
pub mod lexer;
pub mod model;
pub mod parser;
pub mod reconciler;

// Public API re-exports
pub use compiler::compile_spec;
pub use dump::{dump_pcblib, dump_prjpcb, dump_schdoc, dump_schlib};
pub use executor::{apply_spec_pcblib, apply_spec_prjpcb, apply_spec_schdoc, apply_spec_schlib};
pub use import::{ResolvedSpec, resolve_imports};
pub use reconciler::{reconcile_pcblib, reconcile_pcblib_empty, reconcile_prjpcb, reconcile_prjpcb_empty, reconcile_schdoc, reconcile_schdoc_empty, reconcile_schlib, reconcile_schlib_empty};
pub use eval::{
    EvalResult, ScopeStack, SpecError, SpecErrorCode, Value, eval_expr, eval_let_bindings,
    unit_to_internal,
};
pub use eco::{
    EngineeringChangeOrder, EntityChange, EntityKind, EcoSummary, KindSummary,
    PropChange, PropValue,
};
pub use model::{
    ComponentSpec, FootprintMapSpec, FootprintSpec, GraphicProperties, GraphicSpec,
    GraphicType, PadSpec, ParameterSpec, PartSpec, PcbGraphicProperties, PcbGraphicSpec,
    PcbGraphicType, PcbLibSpec, PinPadMap, PinSpec, PrjPcbSpec, ProjectSpec, SchDocSpec,
    SchLibSpec, SpecDomain, SpecModel,
};
