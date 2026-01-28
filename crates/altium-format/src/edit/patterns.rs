// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic pattern engine.
//!
//! Patterns are code-driven schematic snippets that intelligently place passive
//! components, wires, net labels, and power ports relative to existing components.

use serde::Serialize;

use crate::error::{AltiumError, Result};
use crate::records::sch::{PowerObjectStyle, TextOrientations};
use crate::types::CoordPoint;

use super::session::EditSession;
use super::types::{Direction, Orientation};

/// Result from applying a pattern — describes what was placed.
#[derive(Debug, Clone, Serialize)]
pub struct PatternResult {
    /// Pattern ID that was applied.
    pub pattern_id: String,
    /// Human-readable description of what was placed.
    pub description: String,
    /// Designators of components that were placed.
    pub components_placed: Vec<String>,
    /// Net names that were created or used.
    pub nets_used: Vec<String>,
    /// Number of wires added.
    pub wires_added: usize,
}

// ═══════════════════════════════════════════════════════════════════════════
// BUILT-IN PASSIVES LIBRARY
// ═══════════════════════════════════════════════════════════════════════════

/// Ensure built-in passives are available in the library manager.
///
/// Generates a temporary SchLib with R, C, L, D, FB (ferrite bead) symbols
/// and loads it into the session's library manager.
pub fn ensure_builtin_passives(session: &mut EditSession) -> Result<()> {
    // Check if already loaded
    if session.library().find_component("R_PASSIVE").is_some() {
        return Ok(());
    }

    // Generate a temp SchLib file with passives
    let temp_dir = std::env::temp_dir();
    let lib_path = temp_dir.join(format!("altium_builtin_passives_{}.SchLib", uuid::Uuid::new_v4()));

    // Create the library
    crate::ops::schlib::cmd_create(&lib_path)
        .map_err(|e| AltiumError::Parse(format!("Failed to create passives library: {}", e)))?;

    // Generate 2-pin passives: resistor, capacitor, inductor, diode, ferrite bead
    let passives = [
        ("R_PASSIVE", "1:1:passive:left,2:2:passive:right", "Resistor"),
        ("C_PASSIVE", "1:1:passive:left,2:2:passive:right", "Capacitor"),
        ("L_PASSIVE", "1:1:passive:left,2:2:passive:right", "Inductor"),
        ("D_PASSIVE", "1:A:passive:left,2:K:passive:right", "Diode"),
        ("FB_PASSIVE", "1:1:passive:left,2:2:passive:right", "Ferrite Bead"),
    ];

    for (name, pins, desc) in &passives {
        crate::ops::schlib::cmd_gen_ic(
            &lib_path,
            name,
            pins,
            Some(desc.to_string()),
            "200mil",
            "100mil",
            "100mil",
        )
        .map_err(|e| AltiumError::Parse(format!("Failed to generate {}: {}", name, e)))?;
    }

    // Load into session
    session.load_library(&lib_path)?;

    // Clean up temp file
    let _ = std::fs::remove_file(&lib_path);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// COMMON PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

/// Bypass/decoupling capacitor on a power pin.
///
/// Places a capacitor between a component's power pin and ground with short
/// wire stubs and power port symbols.
pub fn bypass_cap(
    session: &mut EditSession,
    component: &str,
    pin: &str,
    value: &str,
    gnd_net: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    // Find the pin location and direction
    let (pin_endpoint, pin_direction) = find_pin_info(session, component, pin)?;

    // Place cap perpendicular to pin direction, offset from pin
    let offset_mils = 200.0;
    let cap_offset = perpendicular_offset(pin_direction, offset_mils);
    let cap_location = CoordPoint::from_mils(
        pin_endpoint.x.to_mils() + pin_direction.unit_vector_f64().0 * offset_mils
            + cap_offset.0,
        pin_endpoint.y.to_mils() + pin_direction.unit_vector_f64().1 * offset_mils
            + cap_offset.1,
    );

    // Place capacitor (horizontal orientation by default)
    let cap_orientation = cap_orientation_for_direction(pin_direction);
    let designator = session.library_mut().next_designator("C");
    let _cap_idx = session.add_component("C_PASSIVE", cap_location, cap_orientation, Some(&designator))?;

    // Get cap pin locations after placement
    let cap_pins = get_component_pin_locations(session, &designator)?;
    let (pin1_loc, pin2_loc) = if cap_pins.len() >= 2 {
        (cap_pins[0].1, cap_pins[1].1)
    } else {
        return Err(AltiumError::Parse("Capacitor has insufficient pins".into()));
    };

    // Wire from the IC pin to the cap's pin 1
    let _wire_idx1 = session.route_wire(pin_endpoint, pin1_loc)?;

    // Add ground power port at cap's pin 2
    let gnd_orient = ground_orientation_for_direction(pin_direction);
    let gnd_wire_end = extend_point(pin2_loc, flip_direction(pin_direction), 100.0);
    let _wire_idx2 = session.add_wire(&[pin2_loc, gnd_wire_end])?;
    let _gnd_idx = session.add_power_port(
        gnd_net,
        gnd_wire_end,
        PowerObjectStyle::Ground,
        gnd_orient,
    )?;

    // Add value as description
    let result = PatternResult {
        pattern_id: "bypass-cap".to_string(),
        description: format!("{} bypass cap on {}.{} to {}", value, component, pin, gnd_net),
        components_placed: vec![designator],
        nets_used: vec![gnd_net.to_string()],
        wires_added: 2,
    };

    Ok(result)
}

/// Pull-up resistor from a signal pin to a power rail.
pub fn pull_up(
    session: &mut EditSession,
    component: &str,
    pin: &str,
    value: &str,
    power_net: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (pin_endpoint, pin_direction) = find_pin_info(session, component, pin)?;

    // Place resistor extending upward/outward from pin
    let r_offset_mils = 200.0;
    let r_location = CoordPoint::from_mils(
        pin_endpoint.x.to_mils() + pin_direction.unit_vector_f64().0 * r_offset_mils,
        pin_endpoint.y.to_mils() + pin_direction.unit_vector_f64().1 * r_offset_mils,
    );

    // Resistor oriented vertically (pin 1 at bottom, pin 2 at top)
    let designator = session.library_mut().next_designator("R");
    let _r_idx = session.add_component("R_PASSIVE", r_location, Orientation::Rotated90, Some(&designator))?;

    let r_pins = get_component_pin_locations(session, &designator)?;
    let (pin1_loc, pin2_loc) = if r_pins.len() >= 2 {
        (r_pins[0].1, r_pins[1].1)
    } else {
        return Err(AltiumError::Parse("Resistor has insufficient pins".into()));
    };

    // Wire from IC pin to resistor pin 1
    session.route_wire(pin_endpoint, pin1_loc)?;

    // Power bar at resistor pin 2
    let power_wire_end = extend_point(pin2_loc, Direction::Up, 100.0);
    session.add_wire(&[pin2_loc, power_wire_end])?;
    session.add_power_port(
        power_net,
        power_wire_end,
        PowerObjectStyle::Bar,
        TextOrientations::NONE,
    )?;

    Ok(PatternResult {
        pattern_id: "pull-up".to_string(),
        description: format!("{} pull-up on {}.{} to {}", value, component, pin, power_net),
        components_placed: vec![designator],
        nets_used: vec![power_net.to_string()],
        wires_added: 2,
    })
}

/// Pull-down resistor from a signal pin to ground.
pub fn pull_down(
    session: &mut EditSession,
    component: &str,
    pin: &str,
    value: &str,
    gnd_net: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (pin_endpoint, pin_direction) = find_pin_info(session, component, pin)?;

    let r_offset_mils = 200.0;
    let r_location = CoordPoint::from_mils(
        pin_endpoint.x.to_mils() + pin_direction.unit_vector_f64().0 * r_offset_mils,
        pin_endpoint.y.to_mils() + pin_direction.unit_vector_f64().1 * r_offset_mils,
    );

    let designator = session.library_mut().next_designator("R");
    let _r_idx = session.add_component("R_PASSIVE", r_location, Orientation::Rotated90, Some(&designator))?;

    let r_pins = get_component_pin_locations(session, &designator)?;
    let (pin1_loc, pin2_loc) = if r_pins.len() >= 2 {
        (r_pins[0].1, r_pins[1].1)
    } else {
        return Err(AltiumError::Parse("Resistor has insufficient pins".into()));
    };

    // Wire from IC pin to resistor pin 1
    session.route_wire(pin_endpoint, pin1_loc)?;

    // Ground at resistor pin 2
    let gnd_wire_end = extend_point(pin2_loc, Direction::Down, 100.0);
    session.add_wire(&[pin2_loc, gnd_wire_end])?;
    session.add_power_port(
        gnd_net,
        gnd_wire_end,
        PowerObjectStyle::Ground,
        TextOrientations::FLIPPED,
    )?;

    Ok(PatternResult {
        pattern_id: "pull-down".to_string(),
        description: format!("{} pull-down on {}.{} to {}", value, component, pin, gnd_net),
        components_placed: vec![designator],
        nets_used: vec![gnd_net.to_string()],
        wires_added: 2,
    })
}

/// Test point — short wire stub with a net label from a pin.
pub fn test_point(
    session: &mut EditSession,
    component: &str,
    pin: &str,
    label: &str,
) -> Result<PatternResult> {
    let (pin_endpoint, pin_direction) = find_pin_info(session, component, pin)?;

    // Short stub
    let stub_end = extend_point(pin_endpoint, pin_direction, 200.0);
    session.add_wire(&[pin_endpoint, stub_end])?;
    session.add_net_label(label, stub_end)?;

    Ok(PatternResult {
        pattern_id: "test-point".to_string(),
        description: format!("Test point '{}' on {}.{}", label, component, pin),
        components_placed: vec![],
        nets_used: vec![label.to_string()],
        wires_added: 1,
    })
}

/// Series resistor between two pins.
pub fn series_resistor(
    session: &mut EditSession,
    from_component: &str,
    from_pin: &str,
    to_component: &str,
    to_pin: &str,
    value: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (from_loc, _) = find_pin_info(session, from_component, from_pin)?;
    let (to_loc, _) = find_pin_info(session, to_component, to_pin)?;

    // Place resistor at midpoint
    let mid = CoordPoint::from_mils(
        (from_loc.x.to_mils() + to_loc.x.to_mils()) / 2.0,
        (from_loc.y.to_mils() + to_loc.y.to_mils()) / 2.0,
    );

    // Determine orientation from the line between pins
    let dx = to_loc.x.to_mils() - from_loc.x.to_mils();
    let dy = to_loc.y.to_mils() - from_loc.y.to_mils();
    let orientation = if dy.abs() > dx.abs() {
        Orientation::Rotated90
    } else {
        Orientation::Normal
    };

    let designator = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", mid, orientation, Some(&designator))?;

    let r_pins = get_component_pin_locations(session, &designator)?;
    let (pin1_loc, pin2_loc) = if r_pins.len() >= 2 {
        (r_pins[0].1, r_pins[1].1)
    } else {
        return Err(AltiumError::Parse("Resistor has insufficient pins".into()));
    };

    // Wire from source pin to R pin 1, R pin 2 to dest pin
    session.route_wire(from_loc, pin1_loc)?;
    session.route_wire(pin2_loc, to_loc)?;

    Ok(PatternResult {
        pattern_id: "series-resistor".to_string(),
        description: format!(
            "{} series resistor between {}.{} and {}.{}",
            value, from_component, from_pin, to_component, to_pin
        ),
        components_placed: vec![designator],
        nets_used: vec![],
        wires_added: 2,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// POWER PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

/// Voltage divider: two resistors between high and low rails with midpoint output.
pub fn voltage_divider(
    session: &mut EditSession,
    high_net: &str,
    low_net: &str,
    r_top_value: &str,
    r_bottom_value: &str,
    output_net: &str,
    x: f64,
    y: f64,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let base = CoordPoint::from_mils(x, y);

    // R_top at top position (vertical)
    let r_top_loc = CoordPoint::from_mils(x, y + 200.0);
    let des_top = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", r_top_loc, Orientation::Rotated90, Some(&des_top))?;

    // R_bottom below (vertical)
    let r_bot_loc = CoordPoint::from_mils(x, y - 200.0);
    let des_bot = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", r_bot_loc, Orientation::Rotated90, Some(&des_bot))?;

    let top_pins = get_component_pin_locations(session, &des_top)?;
    let bot_pins = get_component_pin_locations(session, &des_bot)?;

    if top_pins.len() < 2 || bot_pins.len() < 2 {
        return Err(AltiumError::Parse("Resistor has insufficient pins".into()));
    }

    // Wire R_top pin2 to power (top)
    let vcc_point = extend_point(top_pins[1].1, Direction::Up, 100.0);
    session.add_wire(&[top_pins[1].1, vcc_point])?;
    session.add_power_port(high_net, vcc_point, PowerObjectStyle::Bar, TextOrientations::NONE)?;

    // Wire R_top pin1 to R_bottom pin2 (midpoint)
    session.route_wire(top_pins[0].1, bot_pins[1].1)?;

    // Net label at midpoint
    let mid_point = CoordPoint::from_mils(x + 100.0, y);
    session.add_wire(&[base, mid_point])?;
    session.add_net_label(output_net, mid_point)?;

    // Wire R_bottom pin1 to ground
    let gnd_point = extend_point(bot_pins[0].1, Direction::Down, 100.0);
    session.add_wire(&[bot_pins[0].1, gnd_point])?;
    session.add_power_port(low_net, gnd_point, PowerObjectStyle::Ground, TextOrientations::FLIPPED)?;

    Ok(PatternResult {
        pattern_id: "voltage-divider".to_string(),
        description: format!(
            "Voltage divider {}/{} from {} to {}, output {}",
            r_top_value, r_bottom_value, high_net, low_net, output_net
        ),
        components_placed: vec![des_top, des_bot],
        nets_used: vec![high_net.to_string(), low_net.to_string(), output_net.to_string()],
        wires_added: 4,
    })
}

/// Ferrite bead filter: FB in series with bypass caps on both sides.
pub fn ferrite_filter(
    session: &mut EditSession,
    input_net: &str,
    output_net: &str,
    gnd_net: &str,
    x: f64,
    y: f64,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    // Place ferrite bead horizontally
    let fb_loc = CoordPoint::from_mils(x, y);
    let des_fb = session.library_mut().next_designator("FB");
    session.add_component("FB_PASSIVE", fb_loc, Orientation::Normal, Some(&des_fb))?;

    // Input cap (left of FB)
    let c_in_loc = CoordPoint::from_mils(x - 300.0, y - 200.0);
    let des_cin = session.library_mut().next_designator("C");
    session.add_component("C_PASSIVE", c_in_loc, Orientation::Rotated90, Some(&des_cin))?;

    // Output cap (right of FB)
    let c_out_loc = CoordPoint::from_mils(x + 300.0, y - 200.0);
    let des_cout = session.library_mut().next_designator("C");
    session.add_component("C_PASSIVE", c_out_loc, Orientation::Rotated90, Some(&des_cout))?;

    let fb_pins = get_component_pin_locations(session, &des_fb)?;
    let cin_pins = get_component_pin_locations(session, &des_cin)?;
    let cout_pins = get_component_pin_locations(session, &des_cout)?;

    if fb_pins.len() < 2 || cin_pins.len() < 2 || cout_pins.len() < 2 {
        return Err(AltiumError::Parse("Component has insufficient pins".into()));
    }

    // Input side: net label + wire to FB pin 1 + wire to C_in pin 2
    let in_label_pt = extend_point(fb_pins[0].1, Direction::Left, 200.0);
    session.add_wire(&[in_label_pt, fb_pins[0].1])?;
    session.add_net_label(input_net, in_label_pt)?;
    // Junction where C_in taps the input
    session.route_wire(fb_pins[0].1, cin_pins[1].1)?;

    // Output side: net label + wire from FB pin 2 + wire to C_out pin 2
    let out_label_pt = extend_point(fb_pins[1].1, Direction::Right, 200.0);
    session.add_wire(&[fb_pins[1].1, out_label_pt])?;
    session.add_net_label(output_net, out_label_pt)?;
    session.route_wire(fb_pins[1].1, cout_pins[1].1)?;

    // Ground on both caps
    for cap_pins in [&cin_pins, &cout_pins] {
        let gnd_pt = extend_point(cap_pins[0].1, Direction::Down, 100.0);
        session.add_wire(&[cap_pins[0].1, gnd_pt])?;
        session.add_power_port(gnd_net, gnd_pt, PowerObjectStyle::Ground, TextOrientations::FLIPPED)?;
    }

    Ok(PatternResult {
        pattern_id: "ferrite-filter".to_string(),
        description: format!("Ferrite filter from {} to {} ({})", input_net, output_net, gnd_net),
        components_placed: vec![des_fb, des_cin, des_cout],
        nets_used: vec![input_net.to_string(), output_net.to_string(), gnd_net.to_string()],
        wires_added: 6,
    })
}

/// Bulk decoupling capacitor near a power entry point.
pub fn bulk_decoupling(
    session: &mut EditSession,
    power_net: &str,
    gnd_net: &str,
    x: f64,
    y: f64,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let cap_loc = CoordPoint::from_mils(x, y);
    let designator = session.library_mut().next_designator("C");
    session.add_component("C_PASSIVE", cap_loc, Orientation::Rotated90, Some(&designator))?;

    let pins = get_component_pin_locations(session, &designator)?;
    if pins.len() < 2 {
        return Err(AltiumError::Parse("Capacitor has insufficient pins".into()));
    }

    // Power bar on top
    let vcc_pt = extend_point(pins[1].1, Direction::Up, 100.0);
    session.add_wire(&[pins[1].1, vcc_pt])?;
    session.add_power_port(power_net, vcc_pt, PowerObjectStyle::Bar, TextOrientations::NONE)?;

    // Ground on bottom
    let gnd_pt = extend_point(pins[0].1, Direction::Down, 100.0);
    session.add_wire(&[pins[0].1, gnd_pt])?;
    session.add_power_port(gnd_net, gnd_pt, PowerObjectStyle::Ground, TextOrientations::FLIPPED)?;

    Ok(PatternResult {
        pattern_id: "bulk-decoupling".to_string(),
        description: format!("Bulk decoupling cap on {} / {}", power_net, gnd_net),
        components_placed: vec![designator],
        nets_used: vec![power_net.to_string(), gnd_net.to_string()],
        wires_added: 2,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-SPEED DIGITAL PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

/// Series termination resistor at the source end of a high-speed signal.
pub fn series_termination(
    session: &mut EditSession,
    driver_component: &str,
    driver_pin: &str,
    value: &str,
    net_name: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (pin_endpoint, pin_direction) = find_pin_info(session, driver_component, driver_pin)?;

    // Place resistor inline with pin direction
    let r_offset = 200.0;
    let r_location = CoordPoint::from_mils(
        pin_endpoint.x.to_mils() + pin_direction.unit_vector_f64().0 * r_offset,
        pin_endpoint.y.to_mils() + pin_direction.unit_vector_f64().1 * r_offset,
    );

    let orientation = if pin_direction == Direction::Up || pin_direction == Direction::Down {
        Orientation::Rotated90
    } else {
        Orientation::Normal
    };

    let designator = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", r_location, orientation, Some(&designator))?;

    let r_pins = get_component_pin_locations(session, &designator)?;
    if r_pins.len() < 2 {
        return Err(AltiumError::Parse("Resistor has insufficient pins".into()));
    }

    // Wire from driver pin to R pin 1
    session.route_wire(pin_endpoint, r_pins[0].1)?;

    // Net label on R pin 2 (output side)
    let label_pt = extend_point(r_pins[1].1, pin_direction, 100.0);
    session.add_wire(&[r_pins[1].1, label_pt])?;
    session.add_net_label(net_name, label_pt)?;

    Ok(PatternResult {
        pattern_id: "series-termination".to_string(),
        description: format!("{} series term on {}.{} -> {}", value, driver_component, driver_pin, net_name),
        components_placed: vec![designator],
        nets_used: vec![net_name.to_string()],
        wires_added: 2,
    })
}

/// AC coupling capacitor between two pins.
pub fn ac_coupling(
    session: &mut EditSession,
    from_component: &str,
    from_pin: &str,
    to_component: &str,
    to_pin: &str,
    value: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (from_loc, _) = find_pin_info(session, from_component, from_pin)?;
    let (to_loc, _) = find_pin_info(session, to_component, to_pin)?;

    // Place cap at midpoint
    let mid = CoordPoint::from_mils(
        (from_loc.x.to_mils() + to_loc.x.to_mils()) / 2.0,
        (from_loc.y.to_mils() + to_loc.y.to_mils()) / 2.0,
    );

    let dx = to_loc.x.to_mils() - from_loc.x.to_mils();
    let dy = to_loc.y.to_mils() - from_loc.y.to_mils();
    let orientation = if dy.abs() > dx.abs() {
        Orientation::Rotated90
    } else {
        Orientation::Normal
    };

    let designator = session.library_mut().next_designator("C");
    session.add_component("C_PASSIVE", mid, orientation, Some(&designator))?;

    let c_pins = get_component_pin_locations(session, &designator)?;
    if c_pins.len() < 2 {
        return Err(AltiumError::Parse("Capacitor has insufficient pins".into()));
    }

    session.route_wire(from_loc, c_pins[0].1)?;
    session.route_wire(c_pins[1].1, to_loc)?;

    Ok(PatternResult {
        pattern_id: "ac-coupling".to_string(),
        description: format!(
            "{} AC coupling cap between {}.{} and {}.{}",
            value, from_component, from_pin, to_component, to_pin
        ),
        components_placed: vec![designator],
        nets_used: vec![],
        wires_added: 2,
    })
}

/// Differential pair termination resistor.
pub fn diff_pair_termination(
    session: &mut EditSession,
    component: &str,
    pin_p: &str,
    pin_n: &str,
    value: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (p_loc, p_dir) = find_pin_info(session, component, pin_p)?;
    let (n_loc, n_dir) = find_pin_info(session, component, pin_n)?;

    // Place termination resistor between the two pins, offset outward
    let mid = CoordPoint::from_mils(
        (p_loc.x.to_mils() + n_loc.x.to_mils()) / 2.0 + p_dir.unit_vector_f64().0 * 300.0,
        (p_loc.y.to_mils() + n_loc.y.to_mils()) / 2.0 + p_dir.unit_vector_f64().1 * 300.0,
    );

    // Determine orientation: resistor should span between the two pin extensions
    let dx = n_loc.x.to_mils() - p_loc.x.to_mils();
    let dy = n_loc.y.to_mils() - p_loc.y.to_mils();
    let orientation = if dx.abs() > dy.abs() {
        Orientation::Normal
    } else {
        Orientation::Rotated90
    };

    let designator = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", mid, orientation, Some(&designator))?;

    let r_pins = get_component_pin_locations(session, &designator)?;
    if r_pins.len() < 2 {
        return Err(AltiumError::Parse("Resistor has insufficient pins".into()));
    }

    // Wire stubs from diff pair pins to resistor
    let p_stub = extend_point(p_loc, p_dir, 200.0);
    session.add_wire(&[p_loc, p_stub])?;
    session.route_wire(p_stub, r_pins[0].1)?;

    let n_stub = extend_point(n_loc, n_dir, 200.0);
    session.add_wire(&[n_loc, n_stub])?;
    session.route_wire(n_stub, r_pins[1].1)?;

    Ok(PatternResult {
        pattern_id: "diff-pair-termination".to_string(),
        description: format!(
            "{} diff pair termination on {}.{}/{}.{}",
            value, component, pin_p, component, pin_n
        ),
        components_placed: vec![designator],
        nets_used: vec![],
        wires_added: 4,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// ANALOG PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

/// RC low-pass filter: series R + shunt C to ground.
pub fn rc_lowpass(
    session: &mut EditSession,
    input_component: &str,
    input_pin: &str,
    output_net: &str,
    r_value: &str,
    c_value: &str,
    gnd_net: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (pin_loc, pin_dir) = find_pin_info(session, input_component, input_pin)?;

    // Series resistor inline
    let r_offset = 200.0;
    let r_location = CoordPoint::from_mils(
        pin_loc.x.to_mils() + pin_dir.unit_vector_f64().0 * r_offset,
        pin_loc.y.to_mils() + pin_dir.unit_vector_f64().1 * r_offset,
    );

    let r_orient = if pin_dir == Direction::Up || pin_dir == Direction::Down {
        Orientation::Rotated90
    } else {
        Orientation::Normal
    };

    let des_r = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", r_location, r_orient, Some(&des_r))?;

    let r_pins = get_component_pin_locations(session, &des_r)?;
    if r_pins.len() < 2 {
        return Err(AltiumError::Parse("Resistor has insufficient pins".into()));
    }

    // Wire input to R
    session.route_wire(pin_loc, r_pins[0].1)?;

    // Shunt cap from R output to ground (perpendicular)
    let c_offset = perpendicular_offset(pin_dir, 200.0);
    let c_location = CoordPoint::from_mils(
        r_pins[1].1.x.to_mils() + c_offset.0,
        r_pins[1].1.y.to_mils() + c_offset.1,
    );

    let des_c = session.library_mut().next_designator("C");
    session.add_component("C_PASSIVE", c_location, Orientation::Rotated90, Some(&des_c))?;

    let c_pins = get_component_pin_locations(session, &des_c)?;
    if c_pins.len() < 2 {
        return Err(AltiumError::Parse("Capacitor has insufficient pins".into()));
    }

    // Wire R output to C input
    session.route_wire(r_pins[1].1, c_pins[1].1)?;

    // Ground at C output
    let gnd_pt = extend_point(c_pins[0].1, Direction::Down, 100.0);
    session.add_wire(&[c_pins[0].1, gnd_pt])?;
    session.add_power_port(gnd_net, gnd_pt, PowerObjectStyle::Ground, TextOrientations::FLIPPED)?;

    // Output net label
    let label_pt = extend_point(r_pins[1].1, pin_dir, 100.0);
    session.add_wire(&[r_pins[1].1, label_pt])?;
    session.add_net_label(output_net, label_pt)?;

    Ok(PatternResult {
        pattern_id: "rc-lowpass".to_string(),
        description: format!(
            "RC LPF R={} C={} on {}.{} -> {}",
            r_value, c_value, input_component, input_pin, output_net
        ),
        components_placed: vec![des_r, des_c],
        nets_used: vec![output_net.to_string(), gnd_net.to_string()],
        wires_added: 4,
    })
}

/// Feedback voltage divider for a voltage regulator.
pub fn feedback_divider(
    session: &mut EditSession,
    output_net: &str,
    fb_component: &str,
    fb_pin: &str,
    r_top_value: &str,
    r_bottom_value: &str,
    gnd_net: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (fb_loc, fb_dir) = find_pin_info(session, fb_component, fb_pin)?;

    // Place R_top inline from FB pin outward
    let r_top_loc = CoordPoint::from_mils(
        fb_loc.x.to_mils() + fb_dir.unit_vector_f64().0 * 200.0,
        fb_loc.y.to_mils() + fb_dir.unit_vector_f64().1 * 200.0,
    );

    let des_top = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", r_top_loc, Orientation::Normal, Some(&des_top))?;

    // Place R_bottom below FB pin (vertical)
    let r_bot_loc = CoordPoint::from_mils(
        fb_loc.x.to_mils(),
        fb_loc.y.to_mils() - 300.0,
    );

    let des_bot = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", r_bot_loc, Orientation::Rotated90, Some(&des_bot))?;

    let top_pins = get_component_pin_locations(session, &des_top)?;
    let bot_pins = get_component_pin_locations(session, &des_bot)?;

    if top_pins.len() < 2 || bot_pins.len() < 2 {
        return Err(AltiumError::Parse("Resistor has insufficient pins".into()));
    }

    // Wire FB pin to R_top pin 1
    session.route_wire(fb_loc, top_pins[0].1)?;

    // Wire R_top pin 1 to R_bottom pin 2 (they share the FB node)
    session.route_wire(fb_loc, bot_pins[1].1)?;

    // Net label on R_top pin 2 (output side)
    let out_pt = extend_point(top_pins[1].1, fb_dir, 100.0);
    session.add_wire(&[top_pins[1].1, out_pt])?;
    session.add_net_label(output_net, out_pt)?;

    // Ground on R_bottom pin 1
    let gnd_pt = extend_point(bot_pins[0].1, Direction::Down, 100.0);
    session.add_wire(&[bot_pins[0].1, gnd_pt])?;
    session.add_power_port(gnd_net, gnd_pt, PowerObjectStyle::Ground, TextOrientations::FLIPPED)?;

    Ok(PatternResult {
        pattern_id: "feedback-divider".to_string(),
        description: format!(
            "Feedback divider {}/{} from {} to {}.{}",
            r_top_value, r_bottom_value, output_net, fb_component, fb_pin
        ),
        components_placed: vec![des_top, des_bot],
        nets_used: vec![output_net.to_string(), gnd_net.to_string()],
        wires_added: 4,
    })
}

/// RC snubber across two pins.
pub fn snubber(
    session: &mut EditSession,
    component: &str,
    pin_a: &str,
    pin_b: &str,
    r_value: &str,
    c_value: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (a_loc, a_dir) = find_pin_info(session, component, pin_a)?;
    let (b_loc, _b_dir) = find_pin_info(session, component, pin_b)?;

    // Place R and C in series across pins, offset outward
    let mid_x = (a_loc.x.to_mils() + b_loc.x.to_mils()) / 2.0;
    let mid_y = (a_loc.y.to_mils() + b_loc.y.to_mils()) / 2.0;
    let offset = 300.0;

    let r_location = CoordPoint::from_mils(
        mid_x + a_dir.unit_vector_f64().0 * offset,
        mid_y + a_dir.unit_vector_f64().1 * offset + 100.0,
    );
    let c_location = CoordPoint::from_mils(
        mid_x + a_dir.unit_vector_f64().0 * offset,
        mid_y + a_dir.unit_vector_f64().1 * offset - 100.0,
    );

    let des_r = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", r_location, Orientation::Normal, Some(&des_r))?;

    let des_c = session.library_mut().next_designator("C");
    session.add_component("C_PASSIVE", c_location, Orientation::Normal, Some(&des_c))?;

    let r_pins = get_component_pin_locations(session, &des_r)?;
    let c_pins = get_component_pin_locations(session, &des_c)?;

    if r_pins.len() < 2 || c_pins.len() < 2 {
        return Err(AltiumError::Parse("Component has insufficient pins".into()));
    }

    // Wire R and C in series: A -> R pin1, R pin2 -> C pin1, C pin2 -> B
    let a_stub = extend_point(a_loc, a_dir, 100.0);
    session.add_wire(&[a_loc, a_stub])?;
    session.route_wire(a_stub, r_pins[0].1)?;
    session.route_wire(r_pins[1].1, c_pins[0].1)?;
    let b_stub = extend_point(b_loc, a_dir, 100.0);
    session.add_wire(&[b_loc, b_stub])?;
    session.route_wire(c_pins[1].1, b_stub)?;

    Ok(PatternResult {
        pattern_id: "snubber".to_string(),
        description: format!(
            "RC snubber R={} C={} across {}.{} and {}.{}",
            r_value, c_value, component, pin_a, component, pin_b
        ),
        components_placed: vec![des_r, des_c],
        nets_used: vec![],
        wires_added: 5,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// RF PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

/// DC blocking capacitor on an RF signal path.
pub fn dc_block(
    session: &mut EditSession,
    from_component: &str,
    from_pin: &str,
    to_net: &str,
    value: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (pin_loc, pin_dir) = find_pin_info(session, from_component, from_pin)?;

    let c_offset = 200.0;
    let c_location = CoordPoint::from_mils(
        pin_loc.x.to_mils() + pin_dir.unit_vector_f64().0 * c_offset,
        pin_loc.y.to_mils() + pin_dir.unit_vector_f64().1 * c_offset,
    );

    let orientation = if pin_dir == Direction::Up || pin_dir == Direction::Down {
        Orientation::Rotated90
    } else {
        Orientation::Normal
    };

    let designator = session.library_mut().next_designator("C");
    session.add_component("C_PASSIVE", c_location, orientation, Some(&designator))?;

    let c_pins = get_component_pin_locations(session, &designator)?;
    if c_pins.len() < 2 {
        return Err(AltiumError::Parse("Capacitor has insufficient pins".into()));
    }

    session.route_wire(pin_loc, c_pins[0].1)?;

    let label_pt = extend_point(c_pins[1].1, pin_dir, 100.0);
    session.add_wire(&[c_pins[1].1, label_pt])?;
    session.add_net_label(to_net, label_pt)?;

    Ok(PatternResult {
        pattern_id: "dc-block".to_string(),
        description: format!("{} DC block on {}.{} -> {}", value, from_component, from_pin, to_net),
        components_placed: vec![designator],
        nets_used: vec![to_net.to_string()],
        wires_added: 2,
    })
}

/// Pi attenuator: two shunt resistors + one series resistor.
pub fn pi_attenuator(
    session: &mut EditSession,
    input_net: &str,
    output_net: &str,
    r_series_value: &str,
    r_shunt_value: &str,
    gnd_net: &str,
    x: f64,
    y: f64,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    // Series resistor (horizontal center)
    let r_ser_loc = CoordPoint::from_mils(x, y);
    let des_ser = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", r_ser_loc, Orientation::Normal, Some(&des_ser))?;

    // Input shunt resistor (vertical, left side)
    let r_in_loc = CoordPoint::from_mils(x - 300.0, y - 200.0);
    let des_in = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", r_in_loc, Orientation::Rotated90, Some(&des_in))?;

    // Output shunt resistor (vertical, right side)
    let r_out_loc = CoordPoint::from_mils(x + 300.0, y - 200.0);
    let des_out = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", r_out_loc, Orientation::Rotated90, Some(&des_out))?;

    let ser_pins = get_component_pin_locations(session, &des_ser)?;
    let in_pins = get_component_pin_locations(session, &des_in)?;
    let out_pins = get_component_pin_locations(session, &des_out)?;

    if ser_pins.len() < 2 || in_pins.len() < 2 || out_pins.len() < 2 {
        return Err(AltiumError::Parse("Resistor has insufficient pins".into()));
    }

    // Input net label
    let in_label = extend_point(ser_pins[0].1, Direction::Left, 200.0);
    session.add_wire(&[in_label, ser_pins[0].1])?;
    session.add_net_label(input_net, in_label)?;

    // Output net label
    let out_label = extend_point(ser_pins[1].1, Direction::Right, 200.0);
    session.add_wire(&[ser_pins[1].1, out_label])?;
    session.add_net_label(output_net, out_label)?;

    // Wire input shunt top to series input
    session.route_wire(ser_pins[0].1, in_pins[1].1)?;

    // Wire output shunt top to series output
    session.route_wire(ser_pins[1].1, out_pins[1].1)?;

    // Ground on both shunts
    for shunt_pins in [&in_pins, &out_pins] {
        let gnd_pt = extend_point(shunt_pins[0].1, Direction::Down, 100.0);
        session.add_wire(&[shunt_pins[0].1, gnd_pt])?;
        session.add_power_port(gnd_net, gnd_pt, PowerObjectStyle::Ground, TextOrientations::FLIPPED)?;
    }

    Ok(PatternResult {
        pattern_id: "pi-attenuator".to_string(),
        description: format!(
            "Pi attenuator Rs={} Rp={} from {} to {}",
            r_series_value, r_shunt_value, input_net, output_net
        ),
        components_placed: vec![des_ser, des_in, des_out],
        nets_used: vec![input_net.to_string(), output_net.to_string(), gnd_net.to_string()],
        wires_added: 6,
    })
}

/// Bias tee: inductor to DC supply + series cap to RF.
pub fn bias_tee(
    session: &mut EditSession,
    rf_component: &str,
    rf_pin: &str,
    dc_net: &str,
    rf_net: &str,
    _gnd_net: &str,
    _x: f64,
    _y: f64,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (pin_loc, pin_dir) = find_pin_info(session, rf_component, rf_pin)?;

    // Series cap (RF path, horizontal)
    let c_loc = CoordPoint::from_mils(
        pin_loc.x.to_mils() + pin_dir.unit_vector_f64().0 * 300.0,
        pin_loc.y.to_mils() + pin_dir.unit_vector_f64().1 * 300.0,
    );
    let des_c = session.library_mut().next_designator("C");
    let c_orient = if pin_dir == Direction::Up || pin_dir == Direction::Down {
        Orientation::Rotated90
    } else {
        Orientation::Normal
    };
    session.add_component("C_PASSIVE", c_loc, c_orient, Some(&des_c))?;

    // Shunt inductor (to DC supply, perpendicular)
    let _l_offset = perpendicular_offset(pin_dir, 0.0);
    let l_loc = CoordPoint::from_mils(
        pin_loc.x.to_mils() + pin_dir.unit_vector_f64().0 * 100.0,
        pin_loc.y.to_mils() + 300.0,
    );
    let des_l = session.library_mut().next_designator("L");
    session.add_component("L_PASSIVE", l_loc, Orientation::Rotated90, Some(&des_l))?;

    let c_pins = get_component_pin_locations(session, &des_c)?;
    let l_pins = get_component_pin_locations(session, &des_l)?;

    if c_pins.len() < 2 || l_pins.len() < 2 {
        return Err(AltiumError::Parse("Component has insufficient pins".into()));
    }

    // Wire from RF pin to junction point
    let junction = extend_point(pin_loc, pin_dir, 100.0);
    session.add_wire(&[pin_loc, junction])?;

    // Wire to cap
    session.route_wire(junction, c_pins[0].1)?;

    // Wire to inductor
    session.route_wire(junction, l_pins[0].1)?;

    // RF net label on cap output
    let rf_label = extend_point(c_pins[1].1, pin_dir, 100.0);
    session.add_wire(&[c_pins[1].1, rf_label])?;
    session.add_net_label(rf_net, rf_label)?;

    // DC power on inductor output
    let dc_pt = extend_point(l_pins[1].1, Direction::Up, 100.0);
    session.add_wire(&[l_pins[1].1, dc_pt])?;
    session.add_power_port(dc_net, dc_pt, PowerObjectStyle::Bar, TextOrientations::NONE)?;

    Ok(PatternResult {
        pattern_id: "bias-tee".to_string(),
        description: format!("Bias tee on {}.{}, DC={}, RF={}", rf_component, rf_pin, dc_net, rf_net),
        components_placed: vec![des_c, des_l],
        nets_used: vec![dc_net.to_string(), rf_net.to_string()],
        wires_added: 5,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// PROTECTION PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

/// ESD protection diode from signal to VCC/GND rails.
pub fn esd_clamp(
    session: &mut EditSession,
    signal_component: &str,
    signal_pin: &str,
    vcc_net: &str,
    gnd_net: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (pin_loc, pin_dir) = find_pin_info(session, signal_component, signal_pin)?;

    // Place two diodes: one to VCC (anode=signal), one from GND (cathode=signal)
    let stub = extend_point(pin_loc, pin_dir, 200.0);
    session.add_wire(&[pin_loc, stub])?;

    // Upper diode (signal -> VCC)
    let d_up_loc = CoordPoint::from_mils(stub.x.to_mils(), stub.y.to_mils() + 200.0);
    let des_up = session.library_mut().next_designator("D");
    session.add_component("D_PASSIVE", d_up_loc, Orientation::Rotated90, Some(&des_up))?;

    // Lower diode (GND -> signal)
    let d_dn_loc = CoordPoint::from_mils(stub.x.to_mils(), stub.y.to_mils() - 200.0);
    let des_dn = session.library_mut().next_designator("D");
    session.add_component("D_PASSIVE", d_dn_loc, Orientation::Rotated90, Some(&des_dn))?;

    let up_pins = get_component_pin_locations(session, &des_up)?;
    let dn_pins = get_component_pin_locations(session, &des_dn)?;

    if up_pins.len() < 2 || dn_pins.len() < 2 {
        return Err(AltiumError::Parse("Diode has insufficient pins".into()));
    }

    // Wire stubs from junction to diode pins
    session.route_wire(stub, up_pins[0].1)?;
    session.route_wire(stub, dn_pins[1].1)?;

    // VCC on upper diode cathode
    let vcc_pt = extend_point(up_pins[1].1, Direction::Up, 100.0);
    session.add_wire(&[up_pins[1].1, vcc_pt])?;
    session.add_power_port(vcc_net, vcc_pt, PowerObjectStyle::Bar, TextOrientations::NONE)?;

    // GND on lower diode anode
    let gnd_pt = extend_point(dn_pins[0].1, Direction::Down, 100.0);
    session.add_wire(&[dn_pins[0].1, gnd_pt])?;
    session.add_power_port(gnd_net, gnd_pt, PowerObjectStyle::Ground, TextOrientations::FLIPPED)?;

    Ok(PatternResult {
        pattern_id: "esd-clamp".to_string(),
        description: format!(
            "ESD clamp diodes on {}.{} to {}/{}",
            signal_component, signal_pin, vcc_net, gnd_net
        ),
        components_placed: vec![des_up, des_dn],
        nets_used: vec![vcc_net.to_string(), gnd_net.to_string()],
        wires_added: 5,
    })
}

/// TVS diode across a power rail.
pub fn tvs_diode(
    session: &mut EditSession,
    power_net: &str,
    gnd_net: &str,
    x: f64,
    y: f64,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let loc = CoordPoint::from_mils(x, y);
    let designator = session.library_mut().next_designator("D");
    session.add_component("D_PASSIVE", loc, Orientation::Rotated90, Some(&designator))?;

    let pins = get_component_pin_locations(session, &designator)?;
    if pins.len() < 2 {
        return Err(AltiumError::Parse("Diode has insufficient pins".into()));
    }

    // Power bar on anode
    let vcc_pt = extend_point(pins[1].1, Direction::Up, 100.0);
    session.add_wire(&[pins[1].1, vcc_pt])?;
    session.add_power_port(power_net, vcc_pt, PowerObjectStyle::Bar, TextOrientations::NONE)?;

    // Ground on cathode
    let gnd_pt = extend_point(pins[0].1, Direction::Down, 100.0);
    session.add_wire(&[pins[0].1, gnd_pt])?;
    session.add_power_port(gnd_net, gnd_pt, PowerObjectStyle::Ground, TextOrientations::FLIPPED)?;

    Ok(PatternResult {
        pattern_id: "tvs-diode".to_string(),
        description: format!("TVS diode on {} / {}", power_net, gnd_net),
        components_placed: vec![designator],
        nets_used: vec![power_net.to_string(), gnd_net.to_string()],
        wires_added: 2,
    })
}

/// Current limiting resistor in series.
pub fn current_limit(
    session: &mut EditSession,
    source_component: &str,
    source_pin: &str,
    load_net: &str,
    value: &str,
) -> Result<PatternResult> {
    // Equivalent to series_termination but with different naming
    series_termination(session, source_component, source_pin, value, load_net)
        .map(|mut r| {
            r.pattern_id = "current-limit".to_string();
            r.description = format!(
                "{} current limit on {}.{} -> {}",
                value, source_component, source_pin, load_net
            );
            r
        })
}

// ═══════════════════════════════════════════════════════════════════════════
// INTERFACE PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

/// I2C pull-up resistors on SDA and SCL lines.
pub fn i2c_pullups(
    session: &mut EditSession,
    sda_component: &str,
    sda_pin: &str,
    scl_component: &str,
    scl_pin: &str,
    vcc_net: &str,
    value: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    // Pull-up on SDA
    let res_sda = pull_up(session, sda_component, sda_pin, value, vcc_net)?;

    // Pull-up on SCL
    let res_scl = pull_up(session, scl_component, scl_pin, value, vcc_net)?;

    let mut all_components = res_sda.components_placed;
    all_components.extend(res_scl.components_placed);

    Ok(PatternResult {
        pattern_id: "i2c-pullups".to_string(),
        description: format!(
            "{} I2C pull-ups on {}.{}/{}.{} to {}",
            value, sda_component, sda_pin, scl_component, scl_pin, vcc_net
        ),
        components_placed: all_components,
        nets_used: vec![vcc_net.to_string()],
        wires_added: 4,
    })
}

/// Crystal load capacitors on oscillator pins.
pub fn crystal_load_caps(
    session: &mut EditSession,
    component: &str,
    xtal_in_pin: &str,
    xtal_out_pin: &str,
    c_value: &str,
    gnd_net: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (in_loc, in_dir) = find_pin_info(session, component, xtal_in_pin)?;
    let (out_loc, out_dir) = find_pin_info(session, component, xtal_out_pin)?;

    let mut all_designators = Vec::new();

    // Cap on XTAL_IN
    let c_in_loc = CoordPoint::from_mils(
        in_loc.x.to_mils() + in_dir.unit_vector_f64().0 * 200.0,
        in_loc.y.to_mils() - 200.0,
    );
    let des_cin = session.library_mut().next_designator("C");
    session.add_component("C_PASSIVE", c_in_loc, Orientation::Rotated90, Some(&des_cin))?;
    all_designators.push(des_cin.clone());

    let cin_pins = get_component_pin_locations(session, &des_cin)?;
    if cin_pins.len() < 2 {
        return Err(AltiumError::Parse("Capacitor has insufficient pins".into()));
    }

    let in_stub = extend_point(in_loc, in_dir, 100.0);
    session.add_wire(&[in_loc, in_stub])?;
    session.route_wire(in_stub, cin_pins[1].1)?;

    let gnd1 = extend_point(cin_pins[0].1, Direction::Down, 100.0);
    session.add_wire(&[cin_pins[0].1, gnd1])?;
    session.add_power_port(gnd_net, gnd1, PowerObjectStyle::Ground, TextOrientations::FLIPPED)?;

    // Cap on XTAL_OUT
    let c_out_loc = CoordPoint::from_mils(
        out_loc.x.to_mils() + out_dir.unit_vector_f64().0 * 200.0,
        out_loc.y.to_mils() - 200.0,
    );
    let des_cout = session.library_mut().next_designator("C");
    session.add_component("C_PASSIVE", c_out_loc, Orientation::Rotated90, Some(&des_cout))?;
    all_designators.push(des_cout.clone());

    let cout_pins = get_component_pin_locations(session, &des_cout)?;
    if cout_pins.len() < 2 {
        return Err(AltiumError::Parse("Capacitor has insufficient pins".into()));
    }

    let out_stub = extend_point(out_loc, out_dir, 100.0);
    session.add_wire(&[out_loc, out_stub])?;
    session.route_wire(out_stub, cout_pins[1].1)?;

    let gnd2 = extend_point(cout_pins[0].1, Direction::Down, 100.0);
    session.add_wire(&[cout_pins[0].1, gnd2])?;
    session.add_power_port(gnd_net, gnd2, PowerObjectStyle::Ground, TextOrientations::FLIPPED)?;

    Ok(PatternResult {
        pattern_id: "crystal-load-caps".to_string(),
        description: format!(
            "{} crystal load caps on {}.{}/{}.{}",
            c_value, component, xtal_in_pin, component, xtal_out_pin
        ),
        components_placed: all_designators,
        nets_used: vec![gnd_net.to_string()],
        wires_added: 6,
    })
}

/// Reset circuit: RC delay + pull-up on a reset pin.
pub fn reset_circuit(
    session: &mut EditSession,
    component: &str,
    reset_pin: &str,
    vcc_net: &str,
    gnd_net: &str,
    r_value: &str,
    c_value: &str,
) -> Result<PatternResult> {
    ensure_builtin_passives(session)?;

    let (pin_loc, pin_dir) = find_pin_info(session, component, reset_pin)?;

    // Pull-up resistor (vertical, going up)
    let r_loc = CoordPoint::from_mils(
        pin_loc.x.to_mils() + pin_dir.unit_vector_f64().0 * 200.0,
        pin_loc.y.to_mils() + 200.0,
    );
    let des_r = session.library_mut().next_designator("R");
    session.add_component("R_PASSIVE", r_loc, Orientation::Rotated90, Some(&des_r))?;

    // Capacitor to ground (vertical, going down)
    let c_loc = CoordPoint::from_mils(
        pin_loc.x.to_mils() + pin_dir.unit_vector_f64().0 * 200.0,
        pin_loc.y.to_mils() - 200.0,
    );
    let des_c = session.library_mut().next_designator("C");
    session.add_component("C_PASSIVE", c_loc, Orientation::Rotated90, Some(&des_c))?;

    let r_pins = get_component_pin_locations(session, &des_r)?;
    let c_pins = get_component_pin_locations(session, &des_c)?;

    if r_pins.len() < 2 || c_pins.len() < 2 {
        return Err(AltiumError::Parse("Component has insufficient pins".into()));
    }

    // Wire from reset pin to junction
    let junction = extend_point(pin_loc, pin_dir, 200.0);
    session.add_wire(&[pin_loc, junction])?;

    // Wire to R bottom pin
    session.route_wire(junction, r_pins[0].1)?;

    // Wire to C top pin
    session.route_wire(junction, c_pins[1].1)?;

    // VCC on R top
    let vcc_pt = extend_point(r_pins[1].1, Direction::Up, 100.0);
    session.add_wire(&[r_pins[1].1, vcc_pt])?;
    session.add_power_port(vcc_net, vcc_pt, PowerObjectStyle::Bar, TextOrientations::NONE)?;

    // GND on C bottom
    let gnd_pt = extend_point(c_pins[0].1, Direction::Down, 100.0);
    session.add_wire(&[c_pins[0].1, gnd_pt])?;
    session.add_power_port(gnd_net, gnd_pt, PowerObjectStyle::Ground, TextOrientations::FLIPPED)?;

    Ok(PatternResult {
        pattern_id: "reset-circuit".to_string(),
        description: format!(
            "Reset circuit R={} C={} on {}.{} ({}/{})",
            r_value, c_value, component, reset_pin, vcc_net, gnd_net
        ),
        components_placed: vec![des_r, des_c],
        nets_used: vec![vcc_net.to_string(), gnd_net.to_string()],
        wires_added: 5,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Find a pin's endpoint location and direction.
fn find_pin_info(
    session: &EditSession,
    component: &str,
    pin: &str,
) -> Result<(CoordPoint, Direction)> {
    let components = session.layout().get_placed_components(&session.doc.primitives);
    let comp = components
        .iter()
        .find(|c| c.designator == component)
        .ok_or_else(|| AltiumError::Parse(format!("Component not found: {}", component)))?;

    let pin_loc = comp
        .pin_locations
        .iter()
        .find(|p| p.designator == pin || p.name == pin)
        .ok_or_else(|| AltiumError::Parse(format!("Pin not found: {}.{}", component, pin)))?;

    Ok((pin_loc.location, pin_loc.direction))
}

/// Get all pin locations for a component by designator.
fn get_component_pin_locations(
    session: &EditSession,
    designator: &str,
) -> Result<Vec<(String, CoordPoint)>> {
    let components = session.layout().get_placed_components(&session.doc.primitives);
    let comp = components
        .iter()
        .find(|c| c.designator == designator)
        .ok_or_else(|| AltiumError::Parse(format!("Component not found: {}", designator)))?;

    Ok(comp
        .pin_locations
        .iter()
        .map(|p| (p.designator.clone(), p.location))
        .collect())
}

/// Extend a point in a given direction by a distance in mils.
fn extend_point(point: CoordPoint, direction: Direction, distance_mils: f64) -> CoordPoint {
    let (dx, dy) = direction.unit_vector_f64();
    CoordPoint::from_mils(
        point.x.to_mils() + dx * distance_mils,
        point.y.to_mils() + dy * distance_mils,
    )
}

/// Get a perpendicular offset (always tries to go "down" or "right").
fn perpendicular_offset(direction: Direction, distance: f64) -> (f64, f64) {
    match direction {
        Direction::Left | Direction::Right => (0.0, -distance),
        Direction::Up | Direction::Down => (distance, 0.0),
    }
}

/// Flip a direction (Left <-> Right, Up <-> Down).
fn flip_direction(direction: Direction) -> Direction {
    match direction {
        Direction::Left => Direction::Right,
        Direction::Right => Direction::Left,
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
    }
}

/// Choose cap orientation based on pin direction.
fn cap_orientation_for_direction(direction: Direction) -> Orientation {
    match direction {
        Direction::Up | Direction::Down => Orientation::Rotated90,
        Direction::Left | Direction::Right => Orientation::Normal,
    }
}

/// Choose ground power port orientation based on pin direction.
fn ground_orientation_for_direction(direction: Direction) -> TextOrientations {
    match direction {
        Direction::Up | Direction::Right => TextOrientations::FLIPPED,
        Direction::Down | Direction::Left => TextOrientations::NONE,
    }
}
