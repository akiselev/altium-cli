//! DRC report: violation collection and rendering.

use std::collections::BTreeMap;
use std::fmt;

use altium_format_types::pcb::RuleKind;
use autopcb_routes::Point;

use super::{DrcViolation, DrcViolationKind};

/// Comprehensive DRC report.
#[derive(Debug, Clone)]
pub struct DrcReport {
    pub violations: Vec<DrcViolation>,
    pub checked_rule_count: u32,
    pub skipped_rule_count: u32,
    pub skipped_rule_kinds: Vec<RuleKind>,
}

impl DrcReport {
    pub fn new(violations: Vec<DrcViolation>) -> Self {
        Self { violations, checked_rule_count: 0, skipped_rule_count: 0, skipped_rule_kinds: Vec::new() }
    }

    pub fn empty() -> Self {
        Self { violations: Vec::new(), checked_rule_count: 0, skipped_rule_count: 0, skipped_rule_kinds: Vec::new() }
    }

    pub fn with_audit(mut self, checked: u32, skipped_kinds: Vec<RuleKind>) -> Self {
        self.checked_rule_count = checked;
        self.skipped_rule_count = skipped_kinds.len() as u32;
        self.skipped_rule_kinds = skipped_kinds;
        self
    }

    pub fn total_count(&self) -> usize {
        self.violations.len()
    }

    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }

    /// Count violations grouped by rule kind.
    pub fn count_by_rule(&self) -> BTreeMap<RuleKind, usize> {
        let mut map = BTreeMap::new();
        for v in &self.violations {
            *map.entry(v.rule_kind).or_insert(0) += 1;
        }
        map
    }

    /// Count violations grouped by violation kind.
    pub fn count_by_kind(&self) -> BTreeMap<DrcViolationKind, usize> {
        let mut map = BTreeMap::new();
        for v in &self.violations {
            *map.entry(v.kind).or_insert(0) += 1;
        }
        map
    }

    /// Render a summary string for CLI output.
    pub fn render_summary(&self) -> String {
        if self.is_clean() {
            return "DRC: PASS (0 violations)".to_string();
        }
        let mut lines = vec![format!("DRC: FAIL ({} violations)", self.total_count())];
        for (kind, count) in self.count_by_rule() {
            lines.push(format!("  {:?}: {}", kind, count));
        }
        lines.join("\n")
    }

    /// Convert to serializable DrcViolationRecords for RouteSolution storage.
    pub fn to_violation_records(&self) -> Vec<autopcb_routes::DrcViolationRecord> {
        self.violations
            .iter()
            .map(|v| autopcb_routes::DrcViolationRecord {
                kind_name: v.kind.to_string(),
                location: Point { x: v.location.x, y: v.location.y },
                layer: v.layer.map(|l| l.raw()),
                actual_mm: v.actual_mm,
                required_mm: v.required_mm,
                rule_name: v.rule_name.clone(),
            })
            .collect()
    }

    /// Render a categorized table of violation counts per rule kind.
    pub fn render_table(&self) -> String {
        if self.is_clean() {
            return "DRC: PASS (0 violations)".to_string();
        }
        let counts = self.count_by_rule();
        let mut lines = vec![
            format!("DRC: FAIL ({} violations)", self.total_count()),
            String::new(),
            format!("  {:<35} {:>5}", "Rule", "Count"),
            format!("  {:<35} {:>5}", "---", "-----"),
        ];
        for (kind, count) in &counts {
            lines.push(format!("  {:<35} {:>5}", format!("{:?}", kind), count));
        }
        if !self.skipped_rule_kinds.is_empty() {
            lines.push(String::new());
            lines.push(format!("  Skipped rules ({}):", self.skipped_rule_count));
            for kind in &self.skipped_rule_kinds {
                lines.push(format!("    {:?}", kind));
            }
        }
        lines.join("\n")
    }

    /// Render a detailed table showing each violation.
    pub fn render_verbose(&self) -> String {
        if self.is_clean() {
            return "DRC: PASS (0 violations)".to_string();
        }
        let mut lines = vec![format!("DRC: FAIL ({} violations)", self.total_count())];
        lines.push(String::new());
        for (i, v) in self.violations.iter().enumerate() {
            lines.push(format!(
                "  #{}: {} at ({:.4}, {:.4}){} — actual: {:.4} mm, required: {:.4} mm [{}]",
                i + 1,
                v.kind,
                v.location.x,
                v.location.y,
                v.layer.map(|l| format!(" layer {}", l.raw())).unwrap_or_default(),
                v.actual_mm,
                v.required_mm,
                v.rule_name,
            ));
        }
        lines.join("\n")
    }

    /// Convert report to JSON value for machine-readable output.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pass": self.is_clean(),
            "total_violations": self.total_count(),
            "checked_rule_count": self.checked_rule_count,
            "skipped_rule_count": self.skipped_rule_count,
            "skipped_rule_kinds": self.skipped_rule_kinds.iter().map(|k| format!("{:?}", k)).collect::<Vec<_>>(),
            "violations": self.violations.iter().map(|v| {
                serde_json::json!({
                    "kind": v.kind.to_string(),
                    "rule_kind": format!("{:?}", v.rule_kind),
                    "rule_name": &v.rule_name,
                    "location": { "x": v.location.x, "y": v.location.y },
                    "layer": v.layer.map(|l| l.raw()),
                    "actual_mm": v.actual_mm,
                    "required_mm": v.required_mm,
                })
            }).collect::<Vec<_>>(),
            "by_rule": self.count_by_rule().iter().map(|(k, v)| {
                serde_json::json!({ "rule": format!("{:?}", k), "count": v })
            }).collect::<Vec<_>>(),
        })
    }
}

impl fmt::Display for DrcReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render_summary())
    }
}
