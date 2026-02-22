// Core parsing constants from Altium Designer 26.
//
// Sourced from `Altium.Edp.Interfaces.Rt_Schematic.Consts`.
// These values control the binary/text serialization protocol, unit system,
// string encoding, and workspace limits.

// ---------------------------------------------------------------------------
// Unit system
// ---------------------------------------------------------------------------

/// Base unit divisor: 1 mil = 100,000 DXP coordinate units.
///
/// All coordinates in the schematic format are stored as `i32` values in DXP
/// units. To convert to mils: `mils = dxp_value / C_BASE_UNIT`.
/// To convert to mm: `mm = dxp_value as f64 / C_BASE_UNIT as f64 * 0.0254`.
pub const C_BASE_UNIT: i32 = 100_000;

/// Internal precision factor used in coordinate rounding.
///
/// Used by the serializer for sub-mil precision calculations.
pub const C_INTERNAL_PRECISION: i32 = 10_000;

/// Internal metric tolerance for coordinate snapping.
///
/// When converting between metric and imperial coordinates, values within
/// this tolerance (in DXP units) are considered equal.
pub const C_SCH_INTERNAL_TOLERANCE_METRIC: i32 = 20_000;

// ---------------------------------------------------------------------------
// Workspace limits
// ---------------------------------------------------------------------------

/// Maximum workspace coordinate value (in DXP units).
///
/// Coordinates beyond this value are clamped during import.
pub const C_MAX_WORKSPACE_SIZE: i32 = 650_000_000;

/// Minimum workspace coordinate value (in DXP units).
pub const C_MIN_WORKSPACE_SIZE: i32 = 1_000_000;

/// Maximum length of a text parameter string.
pub const C_MAX_TEXT_PARAM_LENGTH: i32 = 32_000;

/// Maximum length of a short string (single-byte length prefix).
///
/// Strings longer than 254 bytes use the extended length encoding.
pub const C_MAX_SHORT_STRING_LENGTH: i32 = 254;

// ---------------------------------------------------------------------------
// String encoding
// ---------------------------------------------------------------------------

/// UTF-8 string prefix marker.
///
/// When a parameter value starts with this prefix, the remainder of the
/// string is UTF-8 encoded. Without this prefix, strings are Windows-1252.
pub const C_SCH_UTF8_PREFIX: &str = "%UTF8%";

/// Special delimiter character used in parametric text format.
///
/// Character 0x8E (142), used as an internal parameter separator in some
/// contexts (e.g., multi-value fields).
pub const C_SCH_SPECIAL_DELIMITER: char = '\u{008e}';

/// Pipe character used as record field delimiter.
///
/// The primary delimiter in the pipe-delimited text format:
/// `|RECORD=1|OwnerIndex=0|...`
pub const C_SCH_VERTICAL_BAR: char = '|';

/// Broken bar character used as alternate delimiter.
///
/// Character 0xA6 (166, `¦`). Used in some contexts as an alternative to
/// the pipe delimiter.
pub const C_SCH_BROKEN_BAR: char = '\u{00a6}';

// ---------------------------------------------------------------------------
// Binary protocol instruction bytes
// ---------------------------------------------------------------------------

/// Binary/embedded stream instruction byte (0xD0 = 208).
///
/// When encountered in the record stream, switches the deserializer to
/// binary mode for reading embedded blob data (e.g., pin sidecar streams).
pub const INSTRUCTION_BINARY: u8 = 0xD0;

/// File stream instruction byte (0xE3 = 227).
///
/// Marks an embedded file stream within the record data.
pub const INSTRUCTION_FILE_STREAM: u8 = 0xE3;

/// Extra object index instruction byte (0xFE = 254).
///
/// When `RECORD` == 254, the next i32 value is the extended record type
/// (`RECORDEX`), allowing record types beyond the 0-253 range.
pub const INSTRUCTION_EXTRA_OBJECT_INDEX: u8 = 0xFE;

/// End-of-instruction-stream marker (0xFF = 255).
///
/// Signals the end of a record stream. The deserializer stops reading
/// records when it encounters this byte.
pub const INSTRUCTION_END: u8 = 0xFF;

// ---------------------------------------------------------------------------
// Metric coordinate lookup table (DXP units for common MM values)
// ---------------------------------------------------------------------------
//
// Pre-computed DXP coordinate values for common metric dimensions.
// Formula: `dxp = round(mm / 0.0254 * 100_000)` = `round(mm * 3_937_007.874)`
//
// These are used for grid snapping, default sizes, and metric presets.

/// 0.25 mm in DXP units.
pub const C_0_25_MM: i32 = 98_425;

/// 0.40 mm in DXP units.
pub const C_0_40_MM: i32 = 157_480;

/// 0.50 mm in DXP units.
pub const C_0_50_MM: i32 = 196_850;

/// 0.75 mm in DXP units.
pub const C_0_75_MM: i32 = 295_275;

/// 1.0 mm in DXP units.
pub const C_1_0_MM: i32 = 393_701;

/// 1.5 mm in DXP units.
pub const C_1_5_MM: i32 = 590_551;

/// 2.0 mm in DXP units.
pub const C_2_0_MM: i32 = 787_402;

/// 2.5 mm in DXP units.
pub const C_2_5_MM: i32 = 984_252;

/// 3.0 mm in DXP units.
pub const C_3_0_MM: i32 = 1_181_102;

/// 3.5 mm in DXP units.
pub const C_3_5_MM: i32 = 1_377_953;

/// 4.0 mm in DXP units.
pub const C_4_0_MM: i32 = 1_574_803;

/// 4.5 mm in DXP units.
pub const C_4_5_MM: i32 = 1_771_654;

/// 5.0 mm in DXP units.
pub const C_5_0_MM: i32 = 1_968_504;

/// 5.5 mm in DXP units.
pub const C_5_5_MM: i32 = 2_165_354;

/// 6.0 mm in DXP units.
pub const C_6_0_MM: i32 = 2_362_205;

/// 6.5 mm in DXP units.
pub const C_6_5_MM: i32 = 2_559_055;

/// 7.0 mm in DXP units.
pub const C_7_0_MM: i32 = 2_755_906;

/// 7.5 mm in DXP units.
pub const C_7_5_MM: i32 = 2_952_756;

/// 8.0 mm in DXP units.
pub const C_8_0_MM: i32 = 3_149_606;

/// 8.5 mm in DXP units.
pub const C_8_5_MM: i32 = 3_346_457;

/// 9.0 mm in DXP units.
pub const C_9_0_MM: i32 = 3_543_307;

/// 9.5 mm in DXP units.
pub const C_9_5_MM: i32 = 3_740_157;

/// 10.0 mm in DXP units.
pub const C_10_0_MM: i32 = 3_937_008;

/// 15.0 mm in DXP units.
pub const C_15_0_MM: i32 = 5_905_512;

/// 20.0 mm in DXP units.
pub const C_20_0_MM: i32 = 7_874_016;

/// 25.0 mm in DXP units.
pub const C_25_0_MM: i32 = 9_842_520;

/// 30.0 mm in DXP units.
pub const C_30_0_MM: i32 = 11_811_024;

/// 35.0 mm in DXP units.
pub const C_35_0_MM: i32 = 13_779_528;

/// 40.0 mm in DXP units.
pub const C_40_0_MM: i32 = 15_748_031;

/// 45.0 mm in DXP units.
pub const C_45_0_MM: i32 = 17_716_535;

/// 50.0 mm in DXP units.
pub const C_50_0_MM: i32 = 19_685_039;

/// 55.0 mm in DXP units.
pub const C_55_0_MM: i32 = 21_653_543;

/// 60.0 mm in DXP units.
pub const C_60_0_MM: i32 = 23_622_047;

/// 65.0 mm in DXP units.
pub const C_65_0_MM: i32 = 25_590_551;

/// 70.0 mm in DXP units.
pub const C_70_0_MM: i32 = 27_559_055;

/// 75.0 mm in DXP units.
pub const C_75_0_MM: i32 = 29_527_559;

/// 80.0 mm in DXP units.
pub const C_80_0_MM: i32 = 31_496_063;

/// 85.0 mm in DXP units.
pub const C_85_0_MM: i32 = 33_464_567;

/// 90.0 mm in DXP units.
pub const C_90_0_MM: i32 = 35_433_071;

/// 95.0 mm in DXP units.
pub const C_95_0_MM: i32 = 37_401_575;

/// 100.0 mm in DXP units.
pub const C_100_0_MM: i32 = 39_370_078;

/// 1000.0 mm in DXP units.
pub const C_1000_0_MM: i32 = 393_700_787;
