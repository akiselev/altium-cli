// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Component categorization logic shared across schdoc and schlib.

/// Categorize a component by its library reference into a human-readable type.
///
/// Returns category name for grouping (Resistor, Capacitor, IC, etc.).
///
/// Library-reference categorization was chosen over designator-prefix matching because Altium
/// components use semantic library names (e.g., 'LPC1768_MCU') but may share designator prefixes
/// (U1, U2). Library-based categorization captures component type accurately whereas designator
/// prefix only indicates generic IC category. Tradeoff: More complex pattern matching, but handles
/// multi-function ICs correctly.
pub fn categorize_component(lib_ref: &str, description: &str) -> &'static str {
    let lib_lower = lib_ref.to_lowercase();
    let desc_lower = description.to_lowercase();

    // ICs and processors
    if lib_lower.contains("fpga") || lib_lower.contains("cpld") {
        return "FPGA/CPLD";
    }
    if lib_lower.contains("mcu")
        || lib_lower.contains("processor")
        || lib_lower.contains("cortex")
        || lib_lower.contains("arm")
        || lib_lower.contains("lpc")
        || desc_lower.contains("mcu")
        || desc_lower.contains("microcontroller")
    {
        return "Microcontroller";
    }
    if lib_lower.contains("ddr")
        || lib_lower.contains("sdram")
        || lib_lower.contains("flash")
        || lib_lower.contains("eeprom")
        || lib_lower.contains("memory")
    {
        return "Memory";
    }
    if lib_lower.contains("adc") || desc_lower.contains("analog-to-digital") {
        return "ADC";
    }
    if lib_lower.contains("dac") || desc_lower.contains("digital-to-analog") {
        return "DAC";
    }
    if lib_lower.contains("phy")
        || lib_lower.contains("ethernet")
        || lib_lower.contains("transceiver")
    {
        return "Transceiver/PHY";
    }
    if lib_lower.contains("pll")
        || lib_lower.contains("clock")
        || lib_lower.contains("oscillator")
        || lib_lower.contains("xtal")
    {
        return "Clock/Oscillator";
    }
    if lib_lower.contains("regulator")
        || lib_lower.contains("ldo")
        || lib_lower.contains("buck")
        || lib_lower.contains("boost")
        || lib_lower.contains("dcdc")
        || lib_lower.contains("power module")
        || desc_lower.contains("regulator")
        || desc_lower.contains("power module")
    {
        return "Power Supply";
    }
    if lib_lower.contains("opamp")
        || lib_lower.contains("op-amp")
        || lib_lower.contains("amplifier")
    {
        return "Amplifier";
    }
    if lib_lower.contains("mux")
        || lib_lower.contains("switch")
        || lib_lower.contains("multiplexer")
    {
        return "Mux/Switch";
    }
    if lib_lower.contains("buffer") || lib_lower.contains("driver") || lib_lower.contains("level") {
        return "Buffer/Driver";
    }

    // Passives
    if lib_lower.contains("capacitor") || (lib_lower.starts_with('c') && lib_lower.len() <= 5) {
        return "Capacitor";
    }
    if lib_lower.contains("resistor") || (lib_lower.starts_with('r') && lib_lower.len() <= 5) {
        return "Resistor";
    }
    if lib_lower.contains("inductor")
        || lib_lower.contains("ferrite")
        || lib_lower.contains("choke")
    {
        return "Inductor/Ferrite";
    }

    // Discrete semiconductors
    if lib_lower.contains("diode") || lib_lower.contains("esd") || lib_lower.contains("tvs") {
        return "Diode/Protection";
    }
    if lib_lower.contains("transistor") || lib_lower.contains("mosfet") || lib_lower.contains("fet")
    {
        return "Transistor";
    }
    if lib_lower.contains("led") {
        return "LED";
    }

    // Connectors
    if lib_lower.contains("connector")
        || lib_lower.contains("header")
        || lib_lower.contains("socket")
        || lib_lower.contains("terminal")
        || lib_lower.contains("jack")
        || lib_lower.contains("usb")
        || lib_lower.contains("rj45")
    {
        return "Connector";
    }

    // Test points
    if lib_lower.contains("test") || lib_lower.contains("tp") {
        return "Test Point";
    }

    "Other IC"
}
