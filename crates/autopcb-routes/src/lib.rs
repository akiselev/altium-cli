use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LayerId(pub u16);

impl LayerId {
    pub fn raw(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NetId(pub u32);

impl NetId {
    pub fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceSegment {
    pub net_id: NetId,
    pub layer: LayerId,
    pub start: Point,
    pub end: Point,
    pub width_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutedVia {
    pub net_id: NetId,
    pub position: Point,
    pub from_layer: LayerId,
    pub to_layer: LayerId,
    pub drill_mm: f64,
    pub annular_ring_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutedNet {
    pub net_id: NetId,
    pub segments: Vec<TraceSegment>,
    pub vias: Vec<RoutedVia>,
    pub routed_length_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingMetrics {
    pub total_length_mm: f64,
    pub total_vias: u32,
    pub unrouted_count: u32,
    pub completion_pct: f64,
    pub drc_violations: u32,
}

impl Default for RoutingMetrics {
    fn default() -> Self {
        RoutingMetrics {
            total_length_mm: 0.0,
            total_vias: 0,
            unrouted_count: 0,
            completion_pct: 0.0,
            drc_violations: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingIterationSnapshot {
    pub iteration: u32,
    pub conflicts: u32,
    pub routed_count: u32,
    pub unrouted_count: u32,
    pub paths: BTreeMap<NetId, Vec<TraceSegment>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteSolution {
    pub version: u32,
    pub nets: BTreeMap<NetId, RoutedNet>,
    pub unrouted: Vec<NetId>,
    pub metrics: RoutingMetrics,
    pub iterations: Vec<RoutingIterationSnapshot>,
}

impl RouteSolution {
    pub fn new() -> Self {
        RouteSolution {
            version: CURRENT_VERSION,
            nets: BTreeMap::new(),
            unrouted: Vec::new(),
            metrics: RoutingMetrics::default(),
            iterations: Vec::new(),
        }
    }
}

impl Default for RouteSolution {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum RoutesError {
    #[error("unsupported route file version {found}, current version is {current}")]
    UnsupportedVersion { found: u32, current: u32 },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("binary deserialization error: {0}")]
    BincodeDecode(#[from] bincode::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn save_binary(solution: &RouteSolution, path: &Path) -> Result<(), RoutesError> {
    let bytes = bincode::serialize(solution)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

pub fn load_binary(path: &Path) -> Result<RouteSolution, RoutesError> {
    let bytes = std::fs::read(path)?;
    let solution: RouteSolution = bincode::deserialize(&bytes)?;
    if solution.version > CURRENT_VERSION {
        return Err(RoutesError::UnsupportedVersion {
            found: solution.version,
            current: CURRENT_VERSION,
        });
    }
    Ok(solution)
}

pub fn save_json(solution: &RouteSolution, path: &Path) -> Result<(), RoutesError> {
    let json = serde_json::to_string_pretty(solution)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_json(path: &Path) -> Result<RouteSolution, RoutesError> {
    let bytes = std::fs::read(path)?;
    let solution: RouteSolution = serde_json::from_slice(&bytes)?;
    if solution.version > CURRENT_VERSION {
        return Err(RoutesError::UnsupportedVersion {
            found: solution.version,
            current: CURRENT_VERSION,
        });
    }
    Ok(solution)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_empty_solution() -> RouteSolution {
        RouteSolution::new()
    }

    fn make_single_net_solution() -> RouteSolution {
        let net_id = NetId(1);
        let layer = LayerId(0);
        let segment = TraceSegment {
            net_id,
            layer,
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.2,
        };
        let routed_net = RoutedNet {
            net_id,
            segments: vec![segment],
            vias: vec![],
            routed_length_mm: 1.0,
        };
        let mut solution = RouteSolution::new();
        solution.nets.insert(net_id, routed_net);
        solution
    }

    fn make_multi_net_solution() -> RouteSolution {
        let mut solution = RouteSolution::new();

        for i in 0u32..3 {
            let net_id = NetId(i);
            let layer = LayerId(i as u16 % 2);
            let via = RoutedVia {
                net_id,
                position: Point {
                    x: i as f64,
                    y: i as f64,
                },
                from_layer: LayerId(0),
                to_layer: LayerId(1),
                drill_mm: 0.3,
                annular_ring_mm: 0.1,
            };
            let segment = TraceSegment {
                net_id,
                layer,
                start: Point {
                    x: i as f64,
                    y: 0.0,
                },
                end: Point {
                    x: i as f64 + 1.0,
                    y: 0.0,
                },
                width_mm: 0.15,
            };
            let routed_net = RoutedNet {
                net_id,
                segments: vec![segment.clone()],
                vias: vec![via],
                routed_length_mm: 1.0,
            };
            solution.nets.insert(net_id, routed_net);

            let snapshot = RoutingIterationSnapshot {
                iteration: i,
                conflicts: 0,
                routed_count: i + 1,
                unrouted_count: 2 - i,
                paths: BTreeMap::from([(net_id, vec![segment])]),
            };
            solution.iterations.push(snapshot);
        }

        solution.metrics = RoutingMetrics {
            total_length_mm: 3.0,
            total_vias: 3,
            unrouted_count: 0,
            completion_pct: 100.0,
            drc_violations: 0,
        };

        solution
    }

    fn roundtrip_binary(solution: &RouteSolution) -> RouteSolution {
        let tmp = tempfile(solution, "bin");
        load_binary(&tmp).expect("load_binary failed")
    }

    fn roundtrip_json(solution: &RouteSolution) -> RouteSolution {
        let tmp = tempfile(solution, "json");
        load_json(&tmp).expect("load_json failed")
    }

    fn tempfile(solution: &RouteSolution, ext: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir();
        let name = format!(
            "autopcb_routes_test_{}.{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos(),
            ext
        );
        let path = dir.join(name);
        if ext == "bin" {
            save_binary(solution, &path).expect("save_binary failed");
        } else {
            save_json(solution, &path).expect("save_json failed");
        }
        path
    }

    #[test]
    fn binary_roundtrip_empty() {
        let solution = make_empty_solution();
        assert_eq!(roundtrip_binary(&solution), solution);
    }

    #[test]
    fn binary_roundtrip_single_net() {
        let solution = make_single_net_solution();
        assert_eq!(roundtrip_binary(&solution), solution);
    }

    #[test]
    fn binary_roundtrip_multi_net() {
        let solution = make_multi_net_solution();
        assert_eq!(roundtrip_binary(&solution), solution);
    }

    #[test]
    fn json_roundtrip_empty() {
        let solution = make_empty_solution();
        assert_eq!(roundtrip_json(&solution), solution);
    }

    #[test]
    fn json_roundtrip_multi_net() {
        let solution = make_multi_net_solution();
        assert_eq!(roundtrip_json(&solution), solution);
    }

    #[test]
    fn zero_length_segment_roundtrip() {
        let net_id = NetId(0);
        let layer = LayerId(0);
        let segment = TraceSegment {
            net_id,
            layer,
            start: Point { x: 5.0, y: 5.0 },
            end: Point { x: 5.0, y: 5.0 },
            width_mm: 0.1,
        };
        let routed_net = RoutedNet {
            net_id,
            segments: vec![segment],
            vias: vec![],
            routed_length_mm: 0.0,
        };
        let mut solution = RouteSolution::new();
        solution.nets.insert(net_id, routed_net);
        assert_eq!(roundtrip_binary(&solution), solution);
    }

    #[test]
    fn zero_area_via_roundtrip() {
        let net_id = NetId(0);
        let via = RoutedVia {
            net_id,
            position: Point { x: 0.0, y: 0.0 },
            from_layer: LayerId(0),
            to_layer: LayerId(0),
            drill_mm: 0.0,
            annular_ring_mm: 0.0,
        };
        let routed_net = RoutedNet {
            net_id,
            segments: vec![],
            vias: vec![via],
            routed_length_mm: 0.0,
        };
        let mut solution = RouteSolution::new();
        solution.nets.insert(net_id, routed_net);
        assert_eq!(roundtrip_binary(&solution), solution);
    }

    #[test]
    fn empty_iteration_snapshots_roundtrip() {
        let mut solution = RouteSolution::new();
        solution.iterations.push(RoutingIterationSnapshot {
            iteration: 0,
            conflicts: 0,
            routed_count: 0,
            unrouted_count: 0,
            paths: BTreeMap::new(),
        });
        assert_eq!(roundtrip_binary(&solution), solution);
    }

    #[test]
    fn version_mismatch_returns_error_binary() {
        let mut solution = RouteSolution::new();
        solution.version = CURRENT_VERSION + 1;
        let bytes = bincode::serialize(&solution).unwrap();
        let tmp = std::env::temp_dir().join("autopcb_routes_version_test.bin");
        std::fs::write(&tmp, &bytes).unwrap();
        let result = load_binary(&tmp);
        assert!(
            matches!(result, Err(RoutesError::UnsupportedVersion { .. })),
            "expected UnsupportedVersion error, got {:?}",
            result
        );
    }

    #[test]
    fn version_mismatch_returns_error_json() {
        let mut solution = RouteSolution::new();
        solution.version = CURRENT_VERSION + 1;
        let json = serde_json::to_string_pretty(&solution).unwrap();
        let tmp = std::env::temp_dir().join("autopcb_routes_version_test.json");
        std::fs::write(&tmp, &json).unwrap();
        let result = load_json(&tmp);
        assert!(
            matches!(result, Err(RoutesError::UnsupportedVersion { .. })),
            "expected UnsupportedVersion error, got {:?}",
            result
        );
    }
}
