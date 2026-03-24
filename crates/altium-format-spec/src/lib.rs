pub mod annotation;
pub mod ast;
pub mod compiler;
pub mod diagnostic;
#[cfg(feature = "altium-apply")]
pub mod dump;
pub mod eco;
pub mod eval;
#[cfg(feature = "altium-apply")]
pub mod executor;
pub mod formatter;
pub mod import;
pub mod lexer;
pub mod model;
pub mod parser;
#[cfg(feature = "altium-apply")]
pub mod reconciler;
pub mod resolver;
pub mod sync;
pub mod trivia;
pub mod validator;

// Public API re-exports
pub use annotation::{CompiledAnnotation, generate_short_id, validate_short_id};
pub use compiler::{
    compile_imported_pcblibs, compile_imported_schlibs, compile_spec, compile_spec_with_imports,
    compile_spec_with_resolved,
};
#[cfg(feature = "altium-apply")]
pub use dump::{
    IntLibDump, dump_intlib, dump_pcbdoc, dump_pcblib, dump_placement_block, dump_prjpcb,
    dump_schdoc, dump_schlib,
};
pub use eco::{
    EcoSummary, EngineeringChangeOrder, EntityChange, EntityKind, KindSummary, PropChange,
    PropValue,
};
pub use eval::{
    EvalResult, ScopeStack, Severity, Shape, SpecError, SpecErrorCode, Value, eval_expr,
    eval_let_bindings, unit_to_internal,
};
#[cfg(feature = "altium-apply")]
pub use executor::{
    apply_spec_pcbdoc, apply_spec_pcblib, apply_spec_prjpcb, apply_spec_schdoc, apply_spec_schlib,
};
pub use formatter::{FormatConfig, FormatResult, extract_top_level_trivia, format_spec};
pub use import::{ResolvedSpec, resolve_imports};
pub use model::{
    AutoplaceConfig, BoardSpec, ComponentSpec, FootprintMapSpec, FootprintRef, FootprintSpec,
    GraphicProperties, GraphicSpec, GraphicType, PadSpec, ParameterSpec, PartSpec, PcbDocSpec,
    PcbGraphicProperties, PcbGraphicSpec, PcbGraphicType, PcbLibSpec, PinPadMap, PinSpec,
    PlacementClearanceSpec, PlacementConstraintSpec, PlacementGroupSpec, PlacementOptimizeSpec,
    PlacementPlaceSpec, PlacementRuleSpec, PlacementSpec, PrjPcbSpec, ProjectSpec, SchDocSpec,
    SchLibSpec, SpecDomain, SpecModel, UnplacedStrategy,
};
#[cfg(feature = "altium-apply")]
pub use reconciler::{
    reconcile_pcbdoc, reconcile_pcbdoc_empty, reconcile_pcblib, reconcile_pcblib_empty,
    reconcile_prjpcb, reconcile_prjpcb_empty, reconcile_schdoc, reconcile_schdoc_empty,
    reconcile_schlib, reconcile_schlib_empty,
};
pub use resolver::{FootprintResolvedSpec, resolve_schdoc_spec};
pub use sync::{
    FieldChange, SyncChange, SyncComponent, SyncDirection, SyncNet, SyncPin, SyncPolicy,
    SyncSnapshot, apply_sync_changes_to_pcbdoc, diff_snapshots, filter_changes,
    project_pcbdoc_spec, project_schdoc_spec, quote_spec_entity_name, quote_spec_string,
    render_eco_report, rewrite_pcbdoc_spec_with_changes,
};
pub use trivia::{
    CommentToken, ItemTrivia, TriviaLine, TriviaMap, parse_with_trivia, scan_trivia_lines,
};
pub use validator::{validate_pcbdoc_spec, validate_schdoc_spec};
