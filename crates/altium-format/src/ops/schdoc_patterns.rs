// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic pattern commands.
//!
//! High-level commands for applying schematic patterns (bypass caps, pull-ups,
//! voltage dividers, etc.) to schematic documents.

use std::path::{Path, PathBuf};

use crate::edit::patterns::{self, PatternResult};
use crate::edit::EditSession;

/// Parse a unit value or interpret as mils (re-export from schdoc_edit).
fn parse_unit_value_or_mil(s: &str) -> Result<f64, String> {
    let s = s.trim();
    if let Ok((coord, unit)) = crate::types::Unit::parse_with_unit(s) {
        if unit != crate::types::Unit::DxpDefault {
            return Ok(coord.to_mils());
        }
    }
    s.parse::<f64>().map_err(|_| {
        format!(
            "Invalid value '{}': expected number with optional unit (e.g., '100mil', '2.54mm')",
            s
        )
    })
}

/// Apply a bypass capacitor pattern.
pub fn cmd_bypass_cap(
    path: &Path,
    component: &str,
    pin: &str,
    value: &str,
    gnd_net: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::bypass_cap(&mut session, component, pin, value, gnd_net)?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Components: {:?}", result.components_placed);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a pull-up resistor pattern.
pub fn cmd_pull_up(
    path: &Path,
    component: &str,
    pin: &str,
    value: &str,
    power_net: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::pull_up(&mut session, component, pin, value, power_net)?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a pull-down resistor pattern.
pub fn cmd_pull_down(
    path: &Path,
    component: &str,
    pin: &str,
    value: &str,
    gnd_net: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::pull_down(&mut session, component, pin, value, gnd_net)?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a test point pattern.
pub fn cmd_test_point(
    path: &Path,
    component: &str,
    pin: &str,
    label: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::test_point(&mut session, component, pin, label)?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a series resistor pattern.
pub fn cmd_series_resistor(
    path: &Path,
    from_component: &str,
    from_pin: &str,
    to_component: &str,
    to_pin: &str,
    value: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::series_resistor(
        &mut session,
        from_component,
        from_pin,
        to_component,
        to_pin,
        value,
    )?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a voltage divider pattern.
pub fn cmd_voltage_divider(
    path: &Path,
    high_net: &str,
    low_net: &str,
    r_top: &str,
    r_bottom: &str,
    output_net: &str,
    x: &str,
    y: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;
    let mut session = EditSession::open(path)?;
    let result = patterns::voltage_divider(
        &mut session,
        high_net,
        low_net,
        r_top,
        r_bottom,
        output_net,
        x_mils,
        y_mils,
    )?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a ferrite bead filter pattern.
pub fn cmd_ferrite_filter(
    path: &Path,
    input_net: &str,
    output_net: &str,
    gnd_net: &str,
    x: &str,
    y: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;
    let mut session = EditSession::open(path)?;
    let result =
        patterns::ferrite_filter(&mut session, input_net, output_net, gnd_net, x_mils, y_mils)?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a bulk decoupling capacitor pattern.
pub fn cmd_bulk_decoupling(
    path: &Path,
    power_net: &str,
    gnd_net: &str,
    x: &str,
    y: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;
    let mut session = EditSession::open(path)?;
    let result = patterns::bulk_decoupling(&mut session, power_net, gnd_net, x_mils, y_mils)?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a series termination resistor pattern.
pub fn cmd_series_termination(
    path: &Path,
    driver_component: &str,
    driver_pin: &str,
    value: &str,
    net_name: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::series_termination(
        &mut session,
        driver_component,
        driver_pin,
        value,
        net_name,
    )?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply an AC coupling capacitor pattern.
pub fn cmd_ac_coupling(
    path: &Path,
    from_component: &str,
    from_pin: &str,
    to_component: &str,
    to_pin: &str,
    value: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::ac_coupling(
        &mut session,
        from_component,
        from_pin,
        to_component,
        to_pin,
        value,
    )?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a differential pair termination pattern.
pub fn cmd_diff_pair_termination(
    path: &Path,
    component: &str,
    pin_p: &str,
    pin_n: &str,
    value: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result =
        patterns::diff_pair_termination(&mut session, component, pin_p, pin_n, value)?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply an RC low-pass filter pattern.
pub fn cmd_rc_lowpass(
    path: &Path,
    input_component: &str,
    input_pin: &str,
    output_net: &str,
    r_value: &str,
    c_value: &str,
    gnd_net: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::rc_lowpass(
        &mut session,
        input_component,
        input_pin,
        output_net,
        r_value,
        c_value,
        gnd_net,
    )?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a feedback voltage divider pattern.
pub fn cmd_feedback_divider(
    path: &Path,
    output_net: &str,
    fb_component: &str,
    fb_pin: &str,
    r_top: &str,
    r_bottom: &str,
    gnd_net: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::feedback_divider(
        &mut session,
        output_net,
        fb_component,
        fb_pin,
        r_top,
        r_bottom,
        gnd_net,
    )?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply an RC snubber pattern.
pub fn cmd_snubber(
    path: &Path,
    component: &str,
    pin_a: &str,
    pin_b: &str,
    r_value: &str,
    c_value: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::snubber(&mut session, component, pin_a, pin_b, r_value, c_value)?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a DC blocking capacitor pattern.
pub fn cmd_dc_block(
    path: &Path,
    from_component: &str,
    from_pin: &str,
    to_net: &str,
    value: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::dc_block(&mut session, from_component, from_pin, to_net, value)?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a pi attenuator pattern.
pub fn cmd_pi_attenuator(
    path: &Path,
    input_net: &str,
    output_net: &str,
    r_series: &str,
    r_shunt: &str,
    gnd_net: &str,
    x: &str,
    y: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;
    let mut session = EditSession::open(path)?;
    let result = patterns::pi_attenuator(
        &mut session,
        input_net,
        output_net,
        r_series,
        r_shunt,
        gnd_net,
        x_mils,
        y_mils,
    )?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply an ESD clamp diode pattern.
pub fn cmd_esd_clamp(
    path: &Path,
    signal_component: &str,
    signal_pin: &str,
    vcc_net: &str,
    gnd_net: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result =
        patterns::esd_clamp(&mut session, signal_component, signal_pin, vcc_net, gnd_net)?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a TVS diode pattern.
pub fn cmd_tvs_diode(
    path: &Path,
    power_net: &str,
    gnd_net: &str,
    x: &str,
    y: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;
    let mut session = EditSession::open(path)?;
    let result = patterns::tvs_diode(&mut session, power_net, gnd_net, x_mils, y_mils)?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply I2C pull-up resistors pattern.
pub fn cmd_i2c_pullups(
    path: &Path,
    sda_component: &str,
    sda_pin: &str,
    scl_component: &str,
    scl_pin: &str,
    vcc_net: &str,
    value: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::i2c_pullups(
        &mut session,
        sda_component,
        sda_pin,
        scl_component,
        scl_pin,
        vcc_net,
        value,
    )?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply crystal load capacitors pattern.
pub fn cmd_crystal_load_caps(
    path: &Path,
    component: &str,
    xtal_in: &str,
    xtal_out: &str,
    c_value: &str,
    gnd_net: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::crystal_load_caps(
        &mut session,
        component,
        xtal_in,
        xtal_out,
        c_value,
        gnd_net,
    )?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// Apply a reset circuit pattern.
pub fn cmd_reset_circuit(
    path: &Path,
    component: &str,
    reset_pin: &str,
    vcc_net: &str,
    gnd_net: &str,
    r_value: &str,
    c_value: &str,
    output: Option<PathBuf>,
) -> Result<PatternResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let result = patterns::reset_circuit(
        &mut session,
        component,
        reset_pin,
        vcc_net,
        gnd_net,
        r_value,
        c_value,
    )?;
    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;
    println!("Applied pattern: {}", result.description);
    println!("Saved to: {}", output_path.display());
    Ok(result)
}

/// List all available patterns.
pub fn cmd_list_patterns() -> Result<(), Box<dyn std::error::Error>> {
    println!("Available SchDoc Patterns:");
    println!();
    println!("  Common:");
    println!("    bypass-cap          Bypass/decoupling capacitor on a power pin");
    println!("    pull-up             Pull-up resistor to power rail");
    println!("    pull-down           Pull-down resistor to ground");
    println!("    test-point          Test point stub with net label");
    println!("    series-resistor     Series resistor between two pins");
    println!();
    println!("  Power:");
    println!("    voltage-divider     Two-resistor voltage divider");
    println!("    ferrite-filter      Ferrite bead with bypass caps");
    println!("    bulk-decoupling     Bulk decoupling capacitor");
    println!();
    println!("  High-Speed Digital:");
    println!("    series-termination  Series termination resistor");
    println!("    ac-coupling         AC coupling capacitor");
    println!("    diff-pair-term      Differential pair termination");
    println!();
    println!("  Analog:");
    println!("    rc-lowpass          RC low-pass filter");
    println!("    feedback-divider    Feedback voltage divider");
    println!("    snubber             RC snubber across pins");
    println!();
    println!("  RF:");
    println!("    dc-block            DC blocking capacitor");
    println!("    pi-attenuator       Pi-network attenuator");
    println!();
    println!("  Protection:");
    println!("    esd-clamp           ESD protection diode pair");
    println!("    tvs-diode           TVS diode on power rail");
    println!();
    println!("  Interface:");
    println!("    i2c-pullups         I2C pull-up resistors");
    println!("    crystal-load-caps   Crystal oscillator load caps");
    println!("    reset-circuit       RC reset circuit with pull-up");
    Ok(())
}
