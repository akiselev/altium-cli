//! Routing engine for automatic wire routing between pins.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::records::sch::SchRecord;
use crate::types::{Coord, CoordPoint, CoordRect};

use super::layout::LayoutEngine;
use super::types::{Direction, Grid, RoutingPath, WireSegment};

/// Cost multipliers for routing decisions.
const STRAIGHT_COST: i64 = 1;
const TURN_COST: i64 = 2;
const CROSSING_COST: i64 = 5;

/// A* node for pathfinding.
#[derive(Clone, Eq, PartialEq)]
struct PathNode {
    position: CoordPoint,
    g_cost: i64, // Actual cost from start
    f_cost: i64, // Estimated total cost (g + heuristic)
    direction: Option<Direction>,
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.f_cost.cmp(&self.f_cost)
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Routing engine for wire connections.
pub struct RoutingEngine {
    /// Grid configuration for wire routing.
    grid: Grid,
    /// Obstacle bounds (components, existing wires).
    obstacles: Vec<CoordRect>,
    /// Existing wire segments.
    existing_wires: Vec<WireSegment>,
    /// Maximum routing iterations.
    max_iterations: usize,
}

impl Default for RoutingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingEngine {
    /// Create a new routing engine.
    pub fn new() -> Self {
        Self {
            grid: Grid::default(),
            obstacles: Vec::new(),
            existing_wires: Vec::new(),
            max_iterations: 10000,
        }
    }

    /// Set the grid configuration.
    pub fn set_grid(&mut self, grid: Grid) {
        self.grid = grid;
    }

    /// Set maximum iterations for routing.
    pub fn set_max_iterations(&mut self, max: usize) {
        self.max_iterations = max;
    }

    /// Update obstacles from schematic primitives.
    pub fn update_obstacles(&mut self, primitives: &[SchRecord], layout: &LayoutEngine) {
        self.obstacles.clear();
        self.existing_wires.clear();

        // Add component bounds as obstacles
        for component in layout.get_placed_components(primitives) {
            // Shrink bounds slightly to allow wires to touch pins
            let margin = Coord::from_mils(5.0);
            self.obstacles.push(CoordRect::from_points(
                component.bounds.location1.x + margin,
                component.bounds.location1.y + margin,
                component.bounds.location2.x - margin,
                component.bounds.location2.y - margin,
            ));
        }

        // Add existing wires
        for record in primitives {
            if let SchRecord::Wire(wire) = record {
                for i in 0..wire.vertices.len().saturating_sub(1) {
                    let start = CoordPoint::from_raw(wire.vertices[i].0, wire.vertices[i].1);
                    let end = CoordPoint::from_raw(wire.vertices[i + 1].0, wire.vertices[i + 1].1);
                    self.existing_wires.push(WireSegment::new(start, end));
                }
            }
        }
    }

    /// Route a wire between two points using A* pathfinding.
    pub fn route(&self, start: CoordPoint, end: CoordPoint) -> Option<RoutingPath> {
        self.route_excluding(start, end, &[])
    }

    /// Route a wire between two points, excluding certain rectangles from obstacles.
    ///
    /// The `exclude_rects` parameter specifies component bounds that should be
    /// ignored during routing (typically the source and target components).
    pub fn route_excluding(
        &self,
        start: CoordPoint,
        end: CoordPoint,
        exclude_rects: &[CoordRect],
    ) -> Option<RoutingPath> {
        let start = self.grid.snap(start);
        let end = self.grid.snap(end);

        // Build filtered obstacle list excluding specified rectangles
        let filtered_obstacles: Vec<CoordRect> = self
            .obstacles
            .iter()
            .filter(|obs| !exclude_rects.iter().any(|ex| rects_overlap(obs, ex)))
            .copied()
            .collect();

        // Quick check for direct path
        if let Some(path) = self.try_direct_path_with(start, end, &filtered_obstacles) {
            return Some(path);
        }

        // Try L-shaped paths (two segments)
        if let Some(path) = self.try_l_path_with(start, end, &filtered_obstacles) {
            return Some(path);
        }

        // Fall back to A* for complex routes
        self.route_astar_with(start, end, &filtered_obstacles)
    }

    /// Try to find a direct horizontal or vertical path.
    fn try_direct_path_with(
        &self,
        start: CoordPoint,
        end: CoordPoint,
        obstacles: &[CoordRect],
    ) -> Option<RoutingPath> {
        let segment = WireSegment::new(start, end);

        // Only works for axis-aligned paths
        if !segment.is_horizontal() && !segment.is_vertical() {
            return None;
        }

        // Check for obstacles
        if self.segment_blocked_by(&segment, obstacles) {
            return None;
        }

        let mut path = RoutingPath::new();
        path.add_segment(segment);
        Some(path)
    }

    /// Try to find an L-shaped path (horizontal then vertical, or vice versa).
    fn try_l_path_with(
        &self,
        start: CoordPoint,
        end: CoordPoint,
        obstacles: &[CoordRect],
    ) -> Option<RoutingPath> {
        // Try horizontal-first L
        let corner1 = CoordPoint::new(end.x, start.y);
        let seg1_h = WireSegment::new(start, corner1);
        let seg2_v = WireSegment::new(corner1, end);

        if !self.segment_blocked_by(&seg1_h, obstacles)
            && !self.segment_blocked_by(&seg2_v, obstacles)
        {
            let mut path = RoutingPath::new();
            path.add_segment(seg1_h);
            path.add_segment(seg2_v);
            return Some(path);
        }

        // Try vertical-first L
        let corner2 = CoordPoint::new(start.x, end.y);
        let seg1_v = WireSegment::new(start, corner2);
        let seg2_h = WireSegment::new(corner2, end);

        if !self.segment_blocked_by(&seg1_v, obstacles)
            && !self.segment_blocked_by(&seg2_h, obstacles)
        {
            let mut path = RoutingPath::new();
            path.add_segment(seg1_v);
            path.add_segment(seg2_h);
            return Some(path);
        }

        None
    }

    /// Check if a segment is blocked by any obstacle in the given list.
    fn segment_blocked_by(&self, segment: &WireSegment, obstacles: &[CoordRect]) -> bool {
        let bounds = segment.bounds();

        for obstacle in obstacles {
            if bounds.intersects(*obstacle) {
                // More precise check for the line segment
                if self.line_intersects_rect(segment, obstacle) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a line segment intersects a rectangle.
    fn line_intersects_rect(&self, segment: &WireSegment, rect: &CoordRect) -> bool {
        let (x1, y1) = (segment.start.x.to_raw(), segment.start.y.to_raw());
        let (x2, y2) = (segment.end.x.to_raw(), segment.end.y.to_raw());
        let (rx1, ry1) = (rect.location1.x.to_raw(), rect.location1.y.to_raw());
        let (rx2, ry2) = (rect.location2.x.to_raw(), rect.location2.y.to_raw());

        // For horizontal/vertical segments, use simple range check
        if segment.is_horizontal() {
            let y = y1;
            if y < ry1 || y > ry2 {
                return false;
            }
            let (min_x, max_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
            return max_x > rx1 && min_x < rx2;
        }

        if segment.is_vertical() {
            let x = x1;
            if x < rx1 || x > rx2 {
                return false;
            }
            let (min_y, max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
            return max_y > ry1 && min_y < ry2;
        }

        // For diagonal segments (shouldn't happen often in schematics)
        // Use Cohen-Sutherland style region checking
        false
    }

    /// Route using A* algorithm with filtered obstacles.
    fn route_astar_with(
        &self,
        start: CoordPoint,
        end: CoordPoint,
        obstacles: &[CoordRect],
    ) -> Option<RoutingPath> {
        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<(i32, i32), ((i32, i32), Direction)> = HashMap::new();
        let mut g_score: HashMap<(i32, i32), i64> = HashMap::new();
        let mut closed_set: HashSet<(i32, i32)> = HashSet::new();

        let start_key = (start.x.to_raw(), start.y.to_raw());
        let end_key = (end.x.to_raw(), end.y.to_raw());

        g_score.insert(start_key, 0);

        open_set.push(PathNode {
            position: start,
            g_cost: 0,
            f_cost: self.heuristic(start, end),
            direction: None,
        });

        let grid_step = self.grid.spacing.to_raw();
        let directions = [
            (Direction::Right, (grid_step, 0)),
            (Direction::Left, (-grid_step, 0)),
            (Direction::Up, (0, grid_step)),
            (Direction::Down, (0, -grid_step)),
        ];

        let mut iterations = 0;

        while let Some(current) = open_set.pop() {
            iterations += 1;
            if iterations > self.max_iterations {
                return None; // Give up if taking too long
            }

            let current_key = (current.position.x.to_raw(), current.position.y.to_raw());

            if current_key == end_key {
                return Some(self.reconstruct_path(came_from, start_key, end_key));
            }

            if closed_set.contains(&current_key) {
                continue;
            }
            closed_set.insert(current_key);

            for (dir, (dx, dy)) in &directions {
                let next_pos = CoordPoint::from_raw(
                    current.position.x.to_raw() + dx,
                    current.position.y.to_raw() + dy,
                );
                let next_key = (next_pos.x.to_raw(), next_pos.y.to_raw());

                if closed_set.contains(&next_key) {
                    continue;
                }

                // Check if this position is blocked (exclude both start and end points)
                if self.point_blocked_by(next_pos, obstacles)
                    && next_key != end_key
                    && next_key != start_key
                {
                    continue;
                }

                // Calculate cost
                let mut move_cost = STRAIGHT_COST;
                if let Some(prev_dir) = current.direction {
                    if prev_dir != *dir {
                        move_cost += TURN_COST;
                    }
                }

                // Check for wire crossings
                let test_segment = WireSegment::new(current.position, next_pos);
                if self.crosses_wire(&test_segment) {
                    move_cost += CROSSING_COST;
                }

                let tentative_g = current.g_cost + move_cost;

                if tentative_g < *g_score.get(&next_key).unwrap_or(&i64::MAX) {
                    came_from.insert(next_key, (current_key, *dir));
                    g_score.insert(next_key, tentative_g);

                    let f_cost = tentative_g + self.heuristic(next_pos, end);
                    open_set.push(PathNode {
                        position: next_pos,
                        g_cost: tentative_g,
                        f_cost,
                        direction: Some(*dir),
                    });
                }
            }
        }

        None // No path found
    }

    /// Manhattan distance heuristic.
    fn heuristic(&self, from: CoordPoint, to: CoordPoint) -> i64 {
        let dx = (to.x.to_raw() - from.x.to_raw()).abs() as i64;
        let dy = (to.y.to_raw() - from.y.to_raw()).abs() as i64;
        dx + dy
    }

    /// Check if a point is inside any obstacle in the given list.
    fn point_blocked_by(&self, point: CoordPoint, obstacles: &[CoordRect]) -> bool {
        for obstacle in obstacles {
            if obstacle.contains(point) {
                return true;
            }
        }
        false
    }

    /// Check if a segment crosses an existing wire.
    fn crosses_wire(&self, segment: &WireSegment) -> bool {
        for wire in &self.existing_wires {
            if self.segments_intersect(segment, wire) {
                return true;
            }
        }
        false
    }

    /// Check if two segments intersect (not just touch at endpoints).
    fn segments_intersect(&self, s1: &WireSegment, s2: &WireSegment) -> bool {
        // Only care about perpendicular crossings
        if (s1.is_horizontal() && s2.is_horizontal()) || (s1.is_vertical() && s2.is_vertical()) {
            return false;
        }

        let (h_seg, v_seg) = if s1.is_horizontal() {
            (s1, s2)
        } else if s1.is_vertical() && s2.is_horizontal() {
            (s2, s1)
        } else {
            return false;
        };

        let h_y = h_seg.start.y.to_raw();
        let h_x1 = h_seg.start.x.to_raw().min(h_seg.end.x.to_raw());
        let h_x2 = h_seg.start.x.to_raw().max(h_seg.end.x.to_raw());

        let v_x = v_seg.start.x.to_raw();
        let v_y1 = v_seg.start.y.to_raw().min(v_seg.end.y.to_raw());
        let v_y2 = v_seg.start.y.to_raw().max(v_seg.end.y.to_raw());

        // Check if they cross (not at endpoints)
        v_x > h_x1 && v_x < h_x2 && h_y > v_y1 && h_y < v_y2
    }

    /// Reconstruct path from A* came_from map.
    fn reconstruct_path(
        &self,
        came_from: HashMap<(i32, i32), ((i32, i32), Direction)>,
        start: (i32, i32),
        end: (i32, i32),
    ) -> RoutingPath {
        let mut path = RoutingPath::new();
        let mut vertices = vec![CoordPoint::from_raw(end.0, end.1)];
        let mut current = end;

        while current != start {
            if let Some((prev, _dir)) = came_from.get(&current) {
                vertices.push(CoordPoint::from_raw(prev.0, prev.1));
                current = *prev;
            } else {
                break;
            }
        }

        vertices.reverse();

        // Simplify path by merging collinear segments
        let simplified = self.simplify_vertices(&vertices);

        for i in 0..simplified.len().saturating_sub(1) {
            path.add_segment(WireSegment::new(simplified[i], simplified[i + 1]));
        }

        path
    }

    /// Simplify vertices by removing collinear intermediate points.
    fn simplify_vertices(&self, vertices: &[CoordPoint]) -> Vec<CoordPoint> {
        if vertices.len() <= 2 {
            return vertices.to_vec();
        }

        let mut simplified = vec![vertices[0]];

        for i in 1..vertices.len() - 1 {
            let prev = simplified.last().unwrap();
            let curr = &vertices[i];
            let next = &vertices[i + 1];

            // Check if curr is collinear with prev and next
            let dx1 = curr.x.to_raw() - prev.x.to_raw();
            let dy1 = curr.y.to_raw() - prev.y.to_raw();
            let dx2 = next.x.to_raw() - curr.x.to_raw();
            let dy2 = next.y.to_raw() - curr.y.to_raw();

            // Not collinear if direction changes
            let collinear = (dx1 == 0 && dx2 == 0) || (dy1 == 0 && dy2 == 0);
            if !collinear {
                simplified.push(*curr);
            }
        }

        simplified.push(*vertices.last().unwrap());
        simplified
    }

    /// Find junctions where new wires intersect existing wires.
    pub fn find_junctions(&self, path: &RoutingPath) -> Vec<CoordPoint> {
        let mut junctions = Vec::new();

        for segment in &path.segments {
            for wire in &self.existing_wires {
                if let Some(intersection) = self.find_intersection(segment, wire) {
                    // Don't add junction at segment endpoints
                    if intersection != segment.start && intersection != segment.end {
                        junctions.push(intersection);
                    }
                }
            }
        }

        // Remove duplicates
        junctions.sort_by_key(|p| (p.x.to_raw(), p.y.to_raw()));
        junctions.dedup_by(|a, b| a.x == b.x && a.y == b.y);

        junctions
    }

    /// Find intersection point of two segments.
    fn find_intersection(&self, s1: &WireSegment, s2: &WireSegment) -> Option<CoordPoint> {
        // Only care about perpendicular intersections
        if (s1.is_horizontal() && s2.is_horizontal()) || (s1.is_vertical() && s2.is_vertical()) {
            return None;
        }

        let (h_seg, v_seg) = if s1.is_horizontal() {
            (s1, s2)
        } else if s1.is_vertical() && s2.is_horizontal() {
            (s2, s1)
        } else {
            return None;
        };

        let h_y = h_seg.start.y;
        let h_x1 = h_seg.start.x.min(h_seg.end.x);
        let h_x2 = h_seg.start.x.max(h_seg.end.x);

        let v_x = v_seg.start.x;
        let v_y1 = v_seg.start.y.min(v_seg.end.y);
        let v_y2 = v_seg.start.y.max(v_seg.end.y);

        // Check if they intersect
        if v_x >= h_x1 && v_x <= h_x2 && h_y >= v_y1 && h_y <= v_y2 {
            Some(CoordPoint::new(v_x, h_y))
        } else {
            None
        }
    }

    /// Auto-route multiple connections, trying to minimize crossings.
    pub fn auto_route_all(
        &mut self,
        connections: &[(CoordPoint, CoordPoint)],
    ) -> Vec<Option<RoutingPath>> {
        let mut results = Vec::new();

        // Sort connections by distance (shorter first)
        let mut sorted_connections: Vec<_> = connections.iter().enumerate().collect();
        sorted_connections.sort_by_key(|(_, (start, end))| {
            let dx = (end.x - start.x).abs().to_raw();
            let dy = (end.y - start.y).abs().to_raw();
            dx + dy
        });

        for (_, (start, end)) in sorted_connections {
            let path = self.route(*start, *end);

            // Add successful route as obstacle for future routes
            if let Some(ref p) = path {
                for segment in &p.segments {
                    self.existing_wires.push(*segment);
                }
            }

            results.push(path);
        }

        results
    }
}

/// Check if two rectangles overlap (used to match component bounds for exclusion).
fn rects_overlap(a: &CoordRect, b: &CoordRect) -> bool {
    a.location1.x == b.location1.x
        && a.location1.y == b.location1.y
        && a.location2.x == b.location2.x
        && a.location2.y == b.location2.y
}
