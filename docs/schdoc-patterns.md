# SchDoc Patterns & Templates

Patterns are code-driven schematic snippets that intelligently place passive components,
wires, net labels, and power ports relative to existing components. Templates are full
schematic sheets that can be instantiated and parameterized.

## Pattern Categories

### Common

| ID | Pattern | Description | Parameters |
|----|---------|-------------|------------|
| `bypass-cap` | Bypass/Decoupling Capacitor | Places a capacitor between a power pin and ground with short traces. Standard practice for every IC power pin. | `component.pin`, `value` (default 100nF), `gnd_net` (default GND) |
| `pull-up` | Pull-Up Resistor | Places a resistor from a signal pin up to a power rail with net labels. | `component.pin`, `value`, `power_net` (default VCC) |
| `pull-down` | Pull-Down Resistor | Places a resistor from a signal pin down to ground. | `component.pin`, `value`, `gnd_net` (default GND) |
| `test-point` | Test Point | Adds a short wire stub with a net label from a pin, marking it as a test point. | `component.pin` or `net`, `label` |
| `no-connect` | No-Connect Flag | Marks unused pins with a no-connect X symbol. | `component.pin` or list of pins |
| `series-resistor` | Series Resistor | Inserts a resistor in series on a signal net between two pins. | `from_pin`, `to_pin`, `value` |
| `net-tie` | Net Tie / 0-Ohm Jumper | Places a 0-ohm resistor connecting two nets, used for star grounding or net bridging. | `net_a`, `net_b`, `location` |
| `label-stub` | Net Label Stub | Short wire stub from a pin with a net label. Alias for `smart-wire` without power. | `component.pin`, `net` |

### Power

| ID | Pattern | Description | Parameters |
|----|---------|-------------|------------|
| `bulk-decoupling` | Bulk Decoupling Capacitor | Places a large electrolytic/tantalum cap near a power entry point. Complements bypass caps. | `power_net`, `gnd_net`, `value` (default 10uF), `location` |
| `voltage-divider` | Voltage Divider | Two resistors in series between a power rail and ground, with a midpoint net label for the divided voltage. | `high_net`, `low_net`, `r_top`, `r_bottom`, `output_net`, `location` |
| `ferrite-filter` | Ferrite Bead Filter | Ferrite bead in series on a power rail with bypass caps on both sides, for isolating analog/digital supplies. | `input_net`, `output_net`, `value`, `location` |
| `power-flag` | Power Rail Flag | Adds a power port symbol (bar/arrow) connected to a net, declaring it as a power source. | `net`, `style` (bar/ground/earth), `location` |
| `rc-power-filter` | RC Power Filter | Series resistor + shunt cap on a power rail for noise-sensitive supplies (ADC VREF, PLL VCC). | `input_net`, `output_net`, `r_value`, `c_value`, `location` |
| `reverse-polarity` | Reverse Polarity Protection | Series or parallel diode on a power input to protect against reversed connections. | `power_net`, `gnd_net`, `location`, `topology` (series/shunt) |

### High-Speed Digital

| ID | Pattern | Description | Parameters |
|----|---------|-------------|------------|
| `series-termination` | Series Termination Resistor | Places a resistor at the source end of a high-speed signal trace. Standard for LVCMOS/LVTTL outputs. | `driver_pin`, `value` (typ. 33ohm), `net` |
| `parallel-termination` | Parallel Termination Resistor | Places a resistor at the receiver end of a signal, terminated to VTT or ground. | `receiver_pin`, `value`, `term_net` |
| `ac-coupling` | AC Coupling Capacitor | Series capacitor on a signal path for DC blocking (HDMI, USB, Ethernet, etc.). | `from_pin`, `to_pin`, `value` |
| `diff-pair-termination` | Differential Pair Termination | Resistor between P/N lines of a differential pair, optionally with two resistors to a center voltage. | `pin_p`, `pin_n`, `r_value`, `topology` (simple/center-tapped), `vtt_net` |
| `diff-pair-ac-coupling` | Differential AC Coupling | Two matched AC coupling caps on a differential pair. | `from_p`, `from_n`, `to_p`, `to_n`, `value` |
| `level-shifter` | Level Shifter (MOSFET) | BSS138 N-FET level shifter with pull-ups on both sides. Classic bidirectional I2C level shift. | `low_pin`, `high_pin`, `low_vcc`, `high_vcc`, `r_value` |
| `bus-termination` | Bus Termination Pack | Resistor network (SIP) terminating a parallel bus (address/data lines). | `pins[]`, `value`, `term_net` |

### Analog

| ID | Pattern | Description | Parameters |
|----|---------|-------------|------------|
| `rc-lowpass` | RC Low-Pass Filter | Series resistor followed by shunt capacitor to ground. Fc = 1/(2*pi*R*C). | `input_pin`, `output_net`, `r_value`, `c_value` |
| `rc-highpass` | RC High-Pass Filter | Series capacitor followed by shunt resistor to ground. | `input_pin`, `output_net`, `c_value`, `r_value` |
| `snubber` | RC Snubber | Series RC across a switch or diode for ringing suppression. | `pin_a`, `pin_b`, `r_value`, `c_value` |
| `feedback-divider` | Feedback Voltage Divider | Two-resistor divider from an output to a feedback pin, with the bottom to ground. Used for voltage regulators. | `output_net`, `fb_pin`, `r_top`, `r_bottom` |
| `input-filter` | Analog Input Filter | RC or LC low-pass filter on an ADC input. | `input_net`, `adc_pin`, `r_value`, `c_value` |
| `gain-resistors` | Op-Amp Gain Set Resistors | Rf/Rg pair for inverting or non-inverting op-amp gain configuration. | `opamp_component`, `rf_value`, `rg_value`, `topology` (inv/non-inv) |
| `reference-bypass` | Voltage Reference Bypass | Close-coupled bypass cap on a precision voltage reference output. | `vref_pin`, `c_value` |

### RF / Mixed-Signal

| ID | Pattern | Description | Parameters |
|----|---------|-------------|------------|
| `dc-block` | DC Blocking Capacitor | Series cap on an RF signal path with controlled impedance placement. | `from_pin`, `to_pin`, `value` |
| `bias-tee` | Bias Tee | Inductor to DC supply + series cap to RF, combining DC bias and RF signal. | `rf_pin`, `dc_net`, `rf_net`, `l_value`, `c_value` |
| `pi-attenuator` | Pi Attenuator | Three-resistor pi-network attenuator for impedance-matched signal level reduction. | `input_pin`, `output_net`, `r_series`, `r_shunt`, `impedance` |
| `t-attenuator` | T Attenuator | Three-resistor T-network attenuator. | `input_pin`, `output_net`, `r_series`, `r_shunt` |
| `matching-network` | L/Pi/T Matching Network | Impedance matching using L/C elements. | `pin`, `topology` (L/Pi/T), `values[]` |
| `balun-interface` | Balun Interface | Balanced-to-unbalanced interface with optional DC bias. | `single_pin`, `diff_p_pin`, `diff_n_pin`, `values[]` |

### Protection / Safety

| ID | Pattern | Description | Parameters |
|----|---------|-------------|------------|
| `esd-clamp` | ESD Protection Diode | TVS or ESD diode from signal to VCC/GND rails. | `signal_pin`, `vcc_net`, `gnd_net` |
| `tvs-diode` | TVS Diode (Power) | Bidirectional TVS across a power rail for surge protection. | `power_net`, `gnd_net`, `location` |
| `current-limit` | Current Limiting Resistor | Series resistor for current limiting (LEDs, GPIO protection). | `source_pin`, `load_net`, `value` |
| `clamping-diodes` | Clamping Diode Pair | Two diodes clamping a signal between VCC and GND rails. | `signal_pin`, `vcc_net`, `gnd_net` |
| `fuse` | Fuse / PTC | Series fuse or resettable PTC on a power input. | `input_net`, `output_net`, `rating`, `location` |
| `spark-gap` | Spark Gap | PCB spark gap for high-voltage transient protection. | `signal_net`, `gnd_net`, `location` |

### Interface / Connector

| ID | Pattern | Description | Parameters |
|----|---------|-------------|------------|
| `i2c-pullups` | I2C Pull-Up Resistors | Matched pull-ups on SDA and SCL lines. | `sda_pin`, `scl_pin`, `vcc_net`, `value` (default 4.7K) |
| `spi-pullups` | SPI CS Pull-Up | Pull-up on chip-select line to keep device deselected by default. | `cs_pin`, `vcc_net`, `value` (default 10K) |
| `uart-protection` | UART Protection | Series resistors on TX/RX for ESD/short protection. | `tx_pin`, `rx_pin`, `value` |
| `reset-circuit` | Reset Circuit (RC) | RC delay + pull-up on a reset pin. Optional manual reset button. | `reset_pin`, `vcc_net`, `r_value`, `c_value` |
| `boot-config` | Boot Config Resistor | Pull-up or pull-down to set boot/config pin state. | `pin`, `state` (high/low), `value`, `supply_net` |
| `crystal-load` | Crystal Load Capacitors | Two matched load caps on crystal oscillator pins to ground. | `xtal_in_pin`, `xtal_out_pin`, `c_value` |
| `jtag-pullups` | JTAG Pull-Ups/Downs | Standard pull-up/down configuration for JTAG pins (TMS, TCK, TDI, TDO, TRST). | `component`, `pins{}`, `vcc_net` |

### Debug / Test

| ID | Pattern | Description | Parameters |
|----|---------|-------------|------------|
| `led-indicator` | LED + Resistor | Series resistor and LED from a signal or power rail to ground. | `source_pin` or `net`, `led_color`, `r_value` |
| `jumper` | Solder Jumper / 0R | Two-pad jumper for configuration or test isolation. | `net_a`, `net_b`, `default` (open/closed) |
| `voltage-probe` | Voltage Probe Point | Resistor divider test point for probing high-voltage rails with an oscilloscope. | `net`, `r_top`, `r_bottom`, `location` |

---

## Templates (Full Schematic Sheets)

Templates are complete `.SchDoc` files that can be instantiated, then customized by
renaming nets and designators.

| ID | Template | Description |
|----|----------|-------------|
| `ldo-3pin` | 3-Pin LDO Regulator | Input cap, output cap, LDO, power flags. |
| `ldo-5pin` | 5-Pin LDO with Enable | Same as above plus enable pull-up, soft-start cap. |
| `buck-converter` | Buck Converter | Bootstrap cap, input/output caps, inductor, feedback divider, compensation. |
| `usb-type-c` | USB Type-C Connector | CC resistors, VBUS protection, ESD, D+/D- routing with AC coupling. |
| `sd-card` | SD Card Interface | Pull-ups on data lines, decoupling, ESD protection. |
| `i2c-bus-sheet` | I2C Bus Sheet | Pull-ups, level shifter, multiple device stubs. |
| `spi-bus-sheet` | SPI Bus Sheet | CS pull-ups, series termination, decoupling per device. |
| `ethernet-phy` | Ethernet PHY Interface | Magnetics, termination, decoupling, LED indicators. |
| `rs485-transceiver` | RS-485 Transceiver | Termination, bias resistors, TVS protection. |
| `can-transceiver` | CAN Bus Transceiver | Termination resistor, common-mode choke, TVS. |
| `power-input` | Power Input Stage | Connector, fuse, TVS, bulk cap, reverse polarity protection. |
| `reset-supervisor` | Reset / Supervisor Circuit | Supervisor IC, decoupling, manual reset button, RC filter. |
| `debug-header` | Debug/JTAG Header | Pin header with pull-up/down configuration, series resistors. |

---

## Implementation Plan

### Phase 1: Pattern Engine Core + Common Patterns

**Goal:** Build the pattern infrastructure and implement the most-used patterns.

1. Add `patterns` module to `altium-format/src/edit/`
   - `PatternConfig` enum with all pattern types and their parameters
   - `PatternResult` describing what was placed (indices, net names, designators)
   - `apply_pattern(&mut EditSession, PatternConfig) -> Result<PatternResult>`

2. Implement Common patterns:
   - `bypass-cap` — most critical, used on every IC
   - `pull-up` / `pull-down`
   - `test-point`
   - `series-resistor`

3. Implement Power patterns:
   - `voltage-divider`
   - `ferrite-filter`
   - `bulk-decoupling`

4. Wire up CLI: `altium-cli schdoc pattern <schDoc> <pattern-id> [args...]`

### Phase 2: High-Speed & Analog Patterns

5. High-Speed Digital:
   - `series-termination`
   - `ac-coupling`
   - `diff-pair-termination`

6. Analog:
   - `rc-lowpass`
   - `feedback-divider`
   - `snubber`

### Phase 3: Remaining Categories

7. RF:
   - `dc-block`
   - `pi-attenuator`
   - `bias-tee`

8. Protection:
   - `esd-clamp`
   - `tvs-diode`
   - `current-limit`

9. Interface:
   - `i2c-pullups`
   - `crystal-load`
   - `reset-circuit`

10. Debug:
    - `led-indicator`
    - `jumper`

### Phase 4: Templates

11. Template engine in `altium-format/src/edit/templates.rs`
12. Parameterized SchDoc generation from embedded or user templates
