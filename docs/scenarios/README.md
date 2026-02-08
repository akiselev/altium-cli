# altium-cli End-to-End Test Scenarios

Twelve scenarios that exercise the full pipeline:

```
datasheet-cli (part selection)
  → schlib create + gen-ic / add-component (schematic symbols)
    → pcblib create + gen-chip / add-dual-row (footprints)
      → schdoc create + edit (schematic entry + wiring)
        → edit validate (ERC)
          → prjpcb create + add-document (project)
            → pcbdoc create + set-outline-rect + add-rule (board setup)
              → prjpcb import-to-pcb (transfer to PCB)
```

Placement and routing are done **manually** after the final step.

## How to read a scenario

Each step has three parts:

1. **`RUN:`** — the exact shell command to execute
2. **`ASSERT:`** — conditions the agent checks on stdout/exit code
   - `exit 0` — process exits successfully
   - `stdout contains "X"` — substring match on stdout
   - `json .field == value` — parse `--json` output, check a field
   - `json .field includes "X"` — array/string membership check
   - `file exists path` — file was created
3. **`MANUAL CHECKPOINT:`** — (rare) operator opens a file in Altium and checks
   one specific visual property. Always written as a single sentence.

If an assertion fails, the agent should report:
```
FAIL scenario 03 step 4.2: expected json .pin_count == 6, got 4
  command: altium-cli schlib component parts.SchLib TMP117 --json
  stdout: { "name": "TMP117", "pin_count": 4, ... }
```

## Scenario index

| # | Name | Parts | Key features tested |
|---|------|-------|---------------------|
| 01 | [Single resistor](01-single-resistor.md) | 1 | Bare-minimum pipeline, gen-chip 0402 |
| 02 | [LED + resistor](02-led-resistor.md) | 2 | Direct wire routing between two components |
| 03 | [MCU power-on](03-mcu-power.md) | 2 | gen-ic, power ports (VCC/GND), bypass cap |
| 04 | [Voltage divider](04-voltage-divider.md) | 3 | Net labels, multiple nets, output tap |
| 05 | [I2C temp sensor](05-i2c-temp-sensor.md) | 5 | I2C pull-ups, bus net labels, header |
| 06 | [Op-amp buffer](06-opamp-buffer.md) | 4 | Analog pins, feedback wire, +/- supply |
| 07 | [MOSFET LED driver](07-mosfet-led-driver.md) | 4 | 3-pin device, gate drive, power switching |
| 08 | [RS-485 transceiver](08-rs485-transceiver.md) | 5 | Differential pair nets, bus termination |
| 09 | [LDO regulator](09-ldo-regulator.md) | 4 | Power input/output rails, enable logic |
| 10 | [SPI flash](10-spi-flash.md) | 6 | SPI bus (4 nets), chip select, pull-ups |
| 11 | [Dual op-amp gain stages](11-dual-opamp.md) | 8 | Multi-section IC, cascaded signal path |
| 12 | [INA219 current sensor](12-current-sensor.md) | 6 | Shunt resistor, I2C, mixed analog/digital |

## Conventions

- All coordinates in **mils**
- Component spacing: ICs 400 mil apart, passives 200 mil apart
- Schematic grid: 50 mil
- PCB grid: 25 mil
- Every scenario creates files in a fresh subdirectory: `work/scenario-NN/`
