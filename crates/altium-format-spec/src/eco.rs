use std::fmt::Write as FmtWrite;
use std::path::PathBuf;
use std::time::SystemTime;

use indexmap::IndexMap;
use serde::Serialize;

// ── Core ECO types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct EngineeringChangeOrder {
    #[serde(serialize_with = "serialize_path")]
    pub library_path: PathBuf,
    #[serde(serialize_with = "serialize_path")]
    pub spec_path: PathBuf,
    #[serde(serialize_with = "serialize_timestamp")]
    pub timestamp: SystemTime,
    pub summary: EcoSummary,
    pub changes: Vec<EntityChange>,
}

fn serialize_path<S>(path: &PathBuf, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&path.display().to_string())
}

fn serialize_timestamp<S>(ts: &SystemTime, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&format_timestamp(*ts))
}

fn format_timestamp(ts: SystemTime) -> String {
    let duration = ts
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Format as UTC: YYYY-MM-DD HH:MM:SS UTC
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    // Simple Julian-day-to-gregorian conversion
    let (year, month, day) = julian_to_gregorian(days_since_epoch as u32 + 2440588);
    format!("{year:04}-{month:02}-{day:02} {h:02}:{m:02}:{s:02} UTC")
}

fn julian_to_gregorian(jd: u32) -> (u32, u32, u32) {
    let a = jd + 32044;
    let b = (4 * a + 3) / 146097;
    let c = a - (146097 * b) / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - (1461 * d) / 4;
    let m = (5 * e + 2) / 153;
    let day = e - (153 * m + 2) / 5 + 1;
    let month = m + 3 - 12 * (m / 10);
    let year = 100 * b + d - 4800 + m / 10;
    (year, month, day)
}

#[derive(Debug, Serialize)]
#[serde(tag = "change", rename_all = "snake_case")]
pub enum EntityChange {
    Add {
        kind: EntityKind,
        identity: String,
        props: Vec<PropValue>,
        children: Vec<EntityChange>,
    },
    Update {
        kind: EntityKind,
        identity: String,
        prop_changes: Vec<PropChange>,
        children: Vec<EntityChange>,
    },
    Unchanged {
        kind: EntityKind,
        identity: String,
    },
}

impl EntityChange {
    pub fn kind(&self) -> EntityKind {
        match self {
            Self::Add { kind, .. } | Self::Update { kind, .. } | Self::Unchanged { kind, .. } => {
                *kind
            }
        }
    }

    pub fn identity(&self) -> &str {
        match self {
            Self::Add { identity, .. }
            | Self::Update { identity, .. }
            | Self::Unchanged { identity, .. } => identity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Component,
    Pin,
    Parameter,
    Alias,
    Graphic,
    Footprint,
    Pad,
    Track,
    Via,
    Arc,
    Text,
    Fill,
    Region,
    // Project
    Project,
    Document,
    OutputGroup,
    OutputJob,
    Variant,
    Variation,
    ComparisonRule,
    ErcMatrixCell,
    ErcLevel,
    // SchDoc
    Sheet,
    Wire,
    Bus,
    NetLabel,
    PowerObject,
    Port,
    Junction,
    NoConnect,
    BusEntry,
    SheetSymbol,
    SheetEntry,
    ParameterSet,
    Note,
    Probe,
    CompileMask,
    Blanket,
    Net,
    Power,
    HarnessConnector,
    SignalHarness,
    // PcbDoc
    Board,
    PcbDocNet,
    PcbDocComponent,
    ComponentBody,
    Polygon,
    Rule,
    Class,
    DifferentialPair,
    Dimension,
}

impl EntityKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Component => "Components",
            Self::Pin => "Pins",
            Self::Parameter => "Parameters",
            Self::Alias => "Aliases",
            Self::Graphic => "Graphics",
            Self::Footprint => "Footprints",
            Self::Pad => "Pads",
            Self::Track => "Tracks",
            Self::Via => "Vias",
            Self::Arc => "Arcs",
            Self::Text => "Texts",
            Self::Fill => "Fills",
            Self::Region => "Regions",
            Self::Project => "Projects",
            Self::Document => "Documents",
            Self::OutputGroup => "Output Groups",
            Self::OutputJob => "Output Jobs",
            Self::Variant => "Variants",
            Self::Variation => "Variations",
            Self::ComparisonRule => "Comparison Rules",
            Self::ErcMatrixCell => "ERC Matrix Cells",
            Self::ErcLevel => "ERC Levels",
            Self::Sheet => "Sheets",
            Self::Wire => "Wires",
            Self::Bus => "Buses",
            Self::NetLabel => "Net Labels",
            Self::PowerObject => "Power Objects",
            Self::Port => "Ports",
            Self::Junction => "Junctions",
            Self::NoConnect => "No Connects",
            Self::BusEntry => "Bus Entries",
            Self::SheetSymbol => "Sheet Symbols",
            Self::SheetEntry => "Sheet Entries",
            Self::ParameterSet => "Parameter Sets",
            Self::Note => "Notes",
            Self::Probe => "Probes",
            Self::CompileMask => "Compile Masks",
            Self::Blanket => "Blankets",
            Self::Net => "Nets",
            Self::Power => "Power Rails",
            Self::HarnessConnector => "Harness Connectors",
            Self::SignalHarness => "Signal Harnesses",
            Self::Board => "Boards",
            Self::PcbDocNet => "Nets",
            Self::PcbDocComponent => "Components",
            Self::ComponentBody => "Component Bodies",
            Self::Polygon => "Polygons",
            Self::Rule => "Rules",
            Self::Class => "Classes",
            Self::DifferentialPair => "Differential Pairs",
            Self::Dimension => "Dimensions",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PropChange {
    pub field: String,
    pub old_value: String,
    pub new_value: String,
}

#[derive(Debug, Serialize)]
pub struct PropValue {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Default, Serialize)]
pub struct EcoSummary {
    pub by_kind: IndexMap<EntityKind, KindSummary>,
}

#[derive(Debug, Default, Serialize)]
pub struct KindSummary {
    pub adds: usize,
    pub updates: usize,
    pub unchanged: usize,
}

// ── Summary computation ──────────────────────────────────────────────────────

pub fn compute_summary(changes: &[EntityChange]) -> EcoSummary {
    let mut summary = EcoSummary::default();
    for change in changes {
        count_change(change, &mut summary);
    }
    summary
}

fn count_change(change: &EntityChange, summary: &mut EcoSummary) {
    match change {
        EntityChange::Add { kind, children, .. } => {
            summary.by_kind.entry(*kind).or_default().adds += 1;
            for child in children {
                count_change(child, summary);
            }
        }
        EntityChange::Update { kind, children, .. } => {
            summary.by_kind.entry(*kind).or_default().updates += 1;
            for child in children {
                count_change(child, summary);
            }
        }
        EntityChange::Unchanged { kind, .. } => {
            summary.by_kind.entry(*kind).or_default().unchanged += 1;
        }
    }
}

// ── Rendering ────────────────────────────────────────────────────────────────

impl EngineeringChangeOrder {
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        render_header(&mut out, self);
        render_summary(&mut out, &self.summary);
        render_changes(&mut out, &self.changes);
        out.push_str("\nEND OF ECO\n");
        out
    }

    pub fn render_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

fn render_header(out: &mut String, eco: &EngineeringChangeOrder) {
    let border = "═".repeat(70);
    writeln!(out, "╔{border}╗").unwrap();
    writeln!(out, "║  ENGINEERING CHANGE ORDER{:>44}║", "").unwrap();
    writeln!(out, "║  Library: {:<60}║", eco.library_path.display()).unwrap();
    writeln!(out, "║  Spec:    {:<60}║", eco.spec_path.display()).unwrap();
    writeln!(out, "║  Date:    {:<60}║", format_timestamp(eco.timestamp)).unwrap();
    writeln!(out, "╚{border}╝").unwrap();
}

fn render_summary(out: &mut String, summary: &EcoSummary) {
    writeln!(out, "\nSUMMARY").unwrap();
    for (kind, counts) in &summary.by_kind {
        if counts.adds > 0 || counts.updates > 0 || counts.unchanged > 0 {
            writeln!(
                out,
                "  {:<14} {} add, {} update, {} unchanged",
                format!("{}:", kind.display_name()),
                counts.adds,
                counts.updates,
                counts.unchanged
            )
            .unwrap();
        }
    }
}

fn render_changes(out: &mut String, changes: &[EntityChange]) {
    writeln!(out, "\nCHANGES").unwrap();

    // Collapse runs of Unchanged at top level
    let mut unchanged_by_kind: IndexMap<EntityKind, usize> = IndexMap::new();
    let mut non_unchanged: Vec<&EntityChange> = Vec::new();

    for change in changes {
        match change {
            EntityChange::Unchanged { kind, .. } => {
                *unchanged_by_kind.entry(*kind).or_default() += 1;
            }
            _ => {
                non_unchanged.push(change);
            }
        }
    }

    for change in &non_unchanged {
        writeln!(out).unwrap();
        render_change(out, change, "  ", true);
    }

    for (kind, count) in &unchanged_by_kind {
        writeln!(
            out,
            "\n  = {count} {} unchanged (not shown)",
            kind.display_name().to_lowercase()
        )
        .unwrap();
    }
}

fn render_change(out: &mut String, change: &EntityChange, indent: &str, _is_top: bool) {
    match change {
        EntityChange::Add {
            kind,
            identity,
            props,
            children,
        } => {
            writeln!(out, "{indent}+ ADD {} \"{identity}\"", kind_label(*kind)).unwrap();
            let prop_indent = format!("{indent}│ ");
            for prop in props {
                writeln!(out, "{prop_indent}{}: \"{}\"", prop.field, prop.value).unwrap();
            }
            render_children(out, children, indent);
        }
        EntityChange::Update {
            kind,
            identity,
            prop_changes,
            children,
        } => {
            writeln!(out, "{indent}~ UPDATE {} \"{identity}\"", kind_label(*kind)).unwrap();
            let prop_indent = format!("{indent}│ ");
            for pc in prop_changes {
                writeln!(
                    out,
                    "{prop_indent}~ {}: \"{}\" → \"{}\"",
                    pc.field, pc.old_value, pc.new_value
                )
                .unwrap();
            }
            render_children(out, children, indent);
        }
        EntityChange::Unchanged { kind, identity } => {
            writeln!(
                out,
                "{indent}= {} \"{identity}\" (unchanged)",
                kind_label(*kind)
            )
            .unwrap();
        }
    }
}

fn render_children(out: &mut String, children: &[EntityChange], parent_indent: &str) {
    // Separate unchanged from non-unchanged children
    let mut unchanged_by_kind: IndexMap<EntityKind, usize> = IndexMap::new();
    let mut shown: Vec<&EntityChange> = Vec::new();

    for child in children {
        match child {
            EntityChange::Unchanged { kind, .. } => {
                *unchanged_by_kind.entry(*kind).or_default() += 1;
            }
            _ => shown.push(child),
        }
    }

    let total = shown.len() + (if unchanged_by_kind.is_empty() { 0 } else { 1 });
    let child_indent = format!("{parent_indent}    ");

    for (i, child) in shown.iter().enumerate() {
        let connector = if i + 1 < total {
            "├──"
        } else {
            "└──"
        };
        let prefix = format!("{parent_indent}{connector} ");
        write!(out, "{prefix}").unwrap();
        render_child_inline(out, child, &child_indent);
    }

    if !unchanged_by_kind.is_empty() {
        let connector = "└──";
        let prefix = format!("{parent_indent}{connector} ");
        for (kind, count) in &unchanged_by_kind {
            writeln!(
                out,
                "{prefix}= {count} {} unchanged",
                kind.display_name().to_lowercase()
            )
            .unwrap();
        }
    }
}

fn render_child_inline(out: &mut String, change: &EntityChange, _indent: &str) {
    match change {
        EntityChange::Add {
            kind,
            identity,
            props,
            ..
        } => {
            write!(out, "+ {} \"{identity}\"", kind_label(*kind)).unwrap();
            for prop in props {
                write!(out, " {}={:?}", prop.field, prop.value).unwrap();
            }
            writeln!(out).unwrap();
        }
        EntityChange::Update {
            kind,
            identity,
            prop_changes,
            ..
        } => {
            write!(out, "~ {} \"{identity}\"", kind_label(*kind)).unwrap();
            for pc in prop_changes {
                write!(out, " {}: {:?}→{:?}", pc.field, pc.old_value, pc.new_value).unwrap();
            }
            writeln!(out).unwrap();
        }
        EntityChange::Unchanged { kind, identity } => {
            writeln!(out, "= {} \"{identity}\" (unchanged)", kind_label(*kind)).unwrap();
        }
    }
}

fn kind_label(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Component => "component",
        EntityKind::Pin => "pin",
        EntityKind::Parameter => "parameter",
        EntityKind::Alias => "alias",
        EntityKind::Graphic => "graphic",
        EntityKind::Footprint => "footprint",
        EntityKind::Pad => "pad",
        EntityKind::Track => "track",
        EntityKind::Via => "via",
        EntityKind::Arc => "arc",
        EntityKind::Text => "text",
        EntityKind::Fill => "fill",
        EntityKind::Region => "region",
        EntityKind::Project => "project",
        EntityKind::Document => "document",
        EntityKind::OutputGroup => "output_group",
        EntityKind::OutputJob => "output",
        EntityKind::Variant => "variant",
        EntityKind::Variation => "variation",
        EntityKind::ComparisonRule => "comparison_rule",
        EntityKind::ErcMatrixCell => "erc_matrix_cell",
        EntityKind::ErcLevel => "erc_level",
        EntityKind::Sheet => "sheet",
        EntityKind::Wire => "wire",
        EntityKind::Bus => "bus",
        EntityKind::NetLabel => "net_label",
        EntityKind::PowerObject => "power_object",
        EntityKind::Port => "port",
        EntityKind::Junction => "junction",
        EntityKind::NoConnect => "no_connect",
        EntityKind::BusEntry => "bus_entry",
        EntityKind::SheetSymbol => "sheet_symbol",
        EntityKind::SheetEntry => "sheet_entry",
        EntityKind::ParameterSet => "parameter_set",
        EntityKind::Note => "note",
        EntityKind::Probe => "probe",
        EntityKind::CompileMask => "compile_mask",
        EntityKind::Blanket => "blanket",
        EntityKind::Net => "net",
        EntityKind::Power => "power",
        EntityKind::HarnessConnector => "harness_connector",
        EntityKind::SignalHarness => "signal_harness",
        EntityKind::Board => "board",
        EntityKind::PcbDocNet => "net",
        EntityKind::PcbDocComponent => "component",
        EntityKind::ComponentBody => "component_body",
        EntityKind::Polygon => "polygon",
        EntityKind::Rule => "rule",
        EntityKind::Class => "class",
        EntityKind::DifferentialPair => "differential_pair",
        EntityKind::Dimension => "dimension",
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_eco(changes: Vec<EntityChange>) -> EngineeringChangeOrder {
        let summary = compute_summary(&changes);
        EngineeringChangeOrder {
            library_path: PathBuf::from("my-parts.SchLib"),
            spec_path: PathBuf::from("my-parts.sym"),
            timestamp: SystemTime::UNIX_EPOCH,
            summary,
            changes,
        }
    }

    #[test]
    fn eco_text_add_only() {
        let changes = vec![EntityChange::Add {
            kind: EntityKind::Component,
            identity: "R_0603".to_string(),
            props: vec![
                PropValue {
                    field: "designator".to_string(),
                    value: "R?".to_string(),
                },
                PropValue {
                    field: "description".to_string(),
                    value: "0603 Resistor".to_string(),
                },
            ],
            children: vec![
                EntityChange::Add {
                    kind: EntityKind::Pin,
                    identity: "1".to_string(),
                    props: vec![],
                    children: vec![],
                },
                EntityChange::Add {
                    kind: EntityKind::Pin,
                    identity: "2".to_string(),
                    props: vec![],
                    children: vec![],
                },
            ],
        }];
        let eco = make_eco(changes);
        let text = eco.render_text();
        assert!(text.contains("ENGINEERING CHANGE ORDER"));
        assert!(text.contains("ADD component \"R_0603\""));
        assert!(text.contains("designator: \"R?\""));
        assert!(text.contains("END OF ECO"));
    }

    #[test]
    fn eco_text_mixed() {
        let changes = vec![
            EntityChange::Add {
                kind: EntityKind::Component,
                identity: "R_NEW".to_string(),
                props: vec![],
                children: vec![],
            },
            EntityChange::Update {
                kind: EntityKind::Component,
                identity: "R_0805".to_string(),
                prop_changes: vec![PropChange {
                    field: "description".to_string(),
                    old_value: "0805 Resistor".to_string(),
                    new_value: "0805 Resistor (updated)".to_string(),
                }],
                children: vec![],
            },
            EntityChange::Unchanged {
                kind: EntityKind::Component,
                identity: "R_0603".to_string(),
            },
        ];
        let eco = make_eco(changes);
        let text = eco.render_text();
        assert!(text.contains("ADD component \"R_NEW\""));
        assert!(text.contains("UPDATE component \"R_0805\""));
        assert!(text.contains("0805 Resistor"));
        assert!(text.contains("0805 Resistor (updated)"));
        assert!(text.contains("1 components unchanged"));
    }

    #[test]
    fn eco_json_rendering() {
        let changes = vec![EntityChange::Add {
            kind: EntityKind::Component,
            identity: "C1".to_string(),
            props: vec![PropValue {
                field: "description".to_string(),
                value: "Capacitor".to_string(),
            }],
            children: vec![],
        }];
        let eco = make_eco(changes);
        let json = eco
            .render_json()
            .expect("JSON serialization should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("Valid JSON");
        assert_eq!(parsed["changes"][0]["change"], "add");
        assert_eq!(parsed["changes"][0]["identity"], "C1");
        assert_eq!(parsed["changes"][0]["kind"], "component");
    }

    #[test]
    fn eco_summary_counts() {
        let changes = vec![
            EntityChange::Add {
                kind: EntityKind::Component,
                identity: "A".to_string(),
                props: vec![],
                children: vec![
                    EntityChange::Add {
                        kind: EntityKind::Pin,
                        identity: "1".to_string(),
                        props: vec![],
                        children: vec![],
                    },
                    EntityChange::Add {
                        kind: EntityKind::Pin,
                        identity: "2".to_string(),
                        props: vec![],
                        children: vec![],
                    },
                ],
            },
            EntityChange::Update {
                kind: EntityKind::Component,
                identity: "B".to_string(),
                prop_changes: vec![],
                children: vec![EntityChange::Unchanged {
                    kind: EntityKind::Pin,
                    identity: "1".to_string(),
                }],
            },
            EntityChange::Unchanged {
                kind: EntityKind::Component,
                identity: "C".to_string(),
            },
        ];
        let summary = compute_summary(&changes);
        let comp = summary.by_kind.get(&EntityKind::Component).unwrap();
        assert_eq!(comp.adds, 1);
        assert_eq!(comp.updates, 1);
        assert_eq!(comp.unchanged, 1);
        let pin = summary.by_kind.get(&EntityKind::Pin).unwrap();
        assert_eq!(pin.adds, 2);
        assert_eq!(pin.unchanged, 1);
    }
}
