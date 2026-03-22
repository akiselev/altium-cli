//! MCP Channel server for the autopcb agent feedback loop.
//!
//! This crate provides the data types for structured solver feedback
//! that an AI agent (Claude Code) can consume to iteratively improve
//! PCB designs.
//!
//! # Architecture
//!
//! ```text
//! Agent writes spec → Compiler → Placer/Router → SensitivityReport
//!      ↑                                              ↓
//!      └──── Agent reads λ values, relaxes binding constraint ←──┘
//! ```
//!
//! The channel pushes `SensitivityReport` events into a running Claude Code
//! session via the MCP `notifications/claude/channel` protocol. The agent
//! can then call a `send_feedback` tool to submit modified constraints.
//!
//! # Phase 1 Scope
//!
//! - [`SensitivityReport`]: JSON-serializable solver output with multipliers
//! - [`ConstraintSensitivity`]: per-constraint feedback (multiplier + residual)
//! - Channel server implementation deferred to Phase 2 (requires MCP SDK)

use serde::{Deserialize, Serialize};

/// Report from the optimizer containing sensitivity information for each constraint.
///
/// This is the primary data structure that the agent consumes to decide which
/// constraints to relax. The agent reads `binding_constraints` sorted by
/// `|multiplier|` and relaxes the most expensive ones first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityReport {
    /// Final objective value after optimization.
    pub objective_value: f64,
    /// Whether the solver converged.
    pub converged: bool,
    /// Total solver iterations (outer + inner).
    pub iterations: usize,
    /// Per-constraint sensitivity information.
    pub constraints: Vec<ConstraintSensitivity>,
    /// Suggested actions for the agent (sorted by impact).
    pub suggestions: Vec<RelaxationSuggestion>,
}

/// Sensitivity information for a single constraint.
///
/// The Lagrange multiplier λ has physical meaning:
/// - For a clearance constraint: "relaxing clearance by 1mm improves HPWL by λ mm"
/// - For a containment constraint: "relaxing board edge by 1mm improves HPWL by λ mm"
/// - Sign: positive λ means the constraint is making the objective worse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintSensitivity {
    /// Human-readable constraint name (e.g., "U1-C3 clearance").
    pub name: String,
    /// Lagrange multiplier value. Larger |λ| = more expensive constraint.
    pub multiplier: f64,
    /// Current constraint residual (0 = exactly satisfied, >0 = violated).
    pub residual: f64,
    /// Whether this constraint is active (binding) at the solution.
    pub is_active: bool,
    /// Whether the user marked this constraint as relaxable.
    pub relaxable: bool,
    /// Priority level (from spec language).
    pub priority: ConstraintPriority,
}

/// Priority level for constraint relaxation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConstraintPriority {
    /// Must not be relaxed.
    High,
    /// Can be relaxed if needed.
    Medium,
    /// Should be relaxed first.
    Low,
}

/// A suggested relaxation action for the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelaxationSuggestion {
    /// Which constraint to relax.
    pub constraint_name: String,
    /// Current value of the constraint bound.
    pub current_bound: f64,
    /// Suggested new bound value.
    pub suggested_bound: f64,
    /// Expected improvement in objective value.
    pub expected_improvement: f64,
    /// Human-readable explanation.
    pub reason: String,
}

impl SensitivityReport {
    /// Create a report indicating the solver did not converge.
    pub fn infeasible(message: &str) -> Self {
        Self {
            objective_value: f64::INFINITY,
            converged: false,
            iterations: 0,
            constraints: Vec::new(),
            suggestions: vec![RelaxationSuggestion {
                constraint_name: "system".to_string(),
                current_bound: 0.0,
                suggested_bound: 0.0,
                expected_improvement: 0.0,
                reason: message.to_string(),
            }],
        }
    }

    /// Sort constraints by |multiplier| descending (most expensive first).
    pub fn binding_constraints(&self) -> Vec<&ConstraintSensitivity> {
        let mut active: Vec<_> = self.constraints.iter().filter(|c| c.is_active).collect();
        active.sort_by(|a, b| {
            b.multiplier
                .abs()
                .partial_cmp(&a.multiplier.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitivity_report_serializes() {
        let report = SensitivityReport {
            objective_value: 183.5,
            converged: true,
            iterations: 42,
            constraints: vec![ConstraintSensitivity {
                name: "U1-C3 clearance".to_string(),
                multiplier: 18.7,
                residual: 0.0,
                is_active: true,
                relaxable: true,
                priority: ConstraintPriority::Low,
            }],
            suggestions: vec![RelaxationSuggestion {
                constraint_name: "U1-C3 clearance".to_string(),
                current_bound: 2.0,
                suggested_bound: 4.0,
                expected_improvement: 37.4,
                reason: "Relaxing from 2mm to 4mm improves HPWL by ~37mm".to_string(),
            }],
        };

        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("U1-C3 clearance"));
        assert!(json.contains("18.7"));

        // Round-trip
        let deserialized: SensitivityReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.objective_value, 183.5);
        assert!(deserialized.converged);
    }

    #[test]
    fn binding_constraints_sorted_by_multiplier() {
        let report = SensitivityReport {
            objective_value: 100.0,
            converged: true,
            iterations: 10,
            constraints: vec![
                ConstraintSensitivity {
                    name: "small".to_string(),
                    multiplier: 1.0,
                    residual: 0.0,
                    is_active: true,
                    relaxable: true,
                    priority: ConstraintPriority::Medium,
                },
                ConstraintSensitivity {
                    name: "large".to_string(),
                    multiplier: 50.0,
                    residual: 0.0,
                    is_active: true,
                    relaxable: true,
                    priority: ConstraintPriority::Low,
                },
                ConstraintSensitivity {
                    name: "inactive".to_string(),
                    multiplier: 0.0,
                    residual: 0.0,
                    is_active: false,
                    relaxable: true,
                    priority: ConstraintPriority::Low,
                },
            ],
            suggestions: vec![],
        };

        let binding = report.binding_constraints();
        assert_eq!(binding.len(), 2); // only active
        assert_eq!(binding[0].name, "large"); // largest first
        assert_eq!(binding[1].name, "small");
    }

    #[test]
    fn infeasible_report() {
        let report = SensitivityReport::infeasible("constraints conflict");
        assert!(!report.converged);
        assert_eq!(report.suggestions.len(), 1);
        assert!(report.suggestions[0].reason.contains("conflict"));
    }
}
