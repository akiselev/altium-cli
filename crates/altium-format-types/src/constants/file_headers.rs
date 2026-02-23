// File format strings and on-disk header strings.
//
// Two categories live here:
//
// 1. **Format strings** -- runtime dispatch tags set on the document object as
//    `DataFormat`. They are *never* written into the file. They are assigned by
//    `FileFormatUtils.GetDataFormatByParameters(TSerializerType, TFileFormatVersion)`.
//
// 2. **Header strings** -- the actual on-disk identifiers written into the
//    `HEADER` parameter of the `FileHeader` OLE stream (binary V5) or as the
//    first line of an ASCII file.

// ---------------------------------------------------------------------------
// Runtime format dispatch strings (never on disk)
// ---------------------------------------------------------------------------

/// ASCII schematic sheet (V4 or V5).
///
/// **Used by:** runtime format dispatch
pub const SCH_FORMAT_STRING_ASCII: &str = "Advanced Schematic ascii(*.asc)";

/// V4 binary schematic sheet.
///
/// **Used by:** runtime format dispatch
pub const SCH_FORMAT_STRING_BINARY_V40: &str = "Advanced Schematic binary v4.0 (*.sch)";

/// V5 binary schematic sheet (OLE2 compound document).
///
/// **Used by:** runtime format dispatch
pub const SCH_FORMAT_STRING_BINARY_V50: &str = "Advanced Schematic binary v5.0 (*.sch)";

/// JSON schematic sheet.
///
/// **Used by:** runtime format dispatch
pub const SCH_FORMAT_STRING_JSON: &str = "Advanced Schematic json(*.json)";

/// ASCII schematic library.
///
/// **Used by:** runtime format dispatch
pub const SCH_FORMAT_STRING_LIBRARY_ASCII: &str = "Advanced Schematic ascii library(*.asc)";

/// V4 binary schematic library.
///
/// **Used by:** runtime format dispatch
pub const SCH_FORMAT_STRING_LIBRARY_BINARY_V40: &str =
    "Advanced Schematic binary library v4.0 (*.lib)";

/// V5 binary schematic library (OLE2 compound document).
///
/// **Used by:** runtime format dispatch
pub const SCH_FORMAT_STRING_LIBRARY_BINARY_V50: &str =
    "Advanced Schematic binary library v5.0 (*.lib)";

// ---------------------------------------------------------------------------
// On-disk schematic sheet headers
// ---------------------------------------------------------------------------

/// V4 binary schematic sheet header (plain binary, NOT OLE2).
///
/// **Era:** Legacy (Protel 98/99/DXP)
/// **Container:** plain binary file
pub const SCH_SHEET_BINARY_HEADER_V40: &str =
    "Protel for Windows - Schematic Capture Binary File Version 1.2 - 2.0";

/// V5 ASCII schematic sheet header.
///
/// **Era:** Current
/// **Container:** plain text file
pub const SCH_SHEET_ASCII_HEADER_V50: &str =
    "Protel for Windows - Schematic Capture Ascii File Version 5.0";

/// V5 binary schematic sheet header (OLE2 compound document).
///
/// **Era:** Current
/// **Container:** OLE2 compound document
pub const SCH_SHEET_BINARY_HEADER_V50: &str =
    "Protel for Windows - Schematic Capture Binary File Version 5.0";

/// V5 JSON schematic sheet header.
///
/// **Era:** Current
/// **Container:** OLE2 compound document
pub const SCH_SHEET_JSON_HEADER_V50: &str =
    "Altium Designer - Schematic Capture Json File Version 5.0";

// ---------------------------------------------------------------------------
// On-disk schematic library headers
// ---------------------------------------------------------------------------

/// V4 ASCII schematic library header.
///
/// **Era:** Legacy (Protel)
/// **Container:** plain text file
pub const SCH_LIBRARY_ASCII_HEADER_V40: &str =
    "Protel for Windows - Schematic Library Editor Ascii File Version 1.2 - 2.0";

/// V4 binary schematic library header.
///
/// **Era:** Legacy (Protel)
/// **Container:** plain binary file
pub const SCH_LIBRARY_BINARY_HEADER_V40: &str =
    "Protel for Windows - Schematic Library Editor Binary File Version 1.2 - 2.0";

/// V5 ASCII schematic library header.
///
/// **Era:** Current
/// **Container:** plain text file
pub const SCH_LIBRARY_ASCII_HEADER_V50: &str =
    "Protel for Windows - Schematic Library Editor Ascii File Version 5.0";

/// V5 binary schematic library header (OLE2 compound document).
///
/// **Era:** Current
/// **Container:** OLE2 compound document
pub const SCH_LIBRARY_BINARY_HEADER_V50: &str =
    "Protel for Windows - Schematic Library Editor Binary File Version 5.0";

/// V5 JSON schematic library header.
///
/// **Era:** Current
/// **Container:** OLE2 compound document
pub const SCH_LIBRARY_JSON_HEADER_V50: &str =
    "Altium Designer - Schematic Library Editor Json File Version 5.0";

// ---------------------------------------------------------------------------
// Harness wiring diagram headers
// ---------------------------------------------------------------------------

/// Harness wiring diagram binary header.
///
/// **Container:** OLE2 compound document
pub const HARNESS_WIRING_DIAGRAM_BINARY_HEADER_V1: &str =
    "Altium Designer - Harness Wiring Diagram Binary File Version 1.0";

/// Harness wiring diagram ASCII header.
///
/// **Container:** plain text file
pub const HARNESS_WIRING_DIAGRAM_ASCII_HEADER_V1: &str =
    "Altium Designer - Harness Wiring Diagram Ascii File Version 1.0";

/// Harness wiring diagram JSON header.
///
/// **Container:** OLE2 compound document
pub const HARNESS_WIRING_DIAGRAM_JSON_HEADER_V1: &str =
    "Altium Designer - Harness Wiring Diagram JSON File Version 1.0";

// ---------------------------------------------------------------------------
// Harness layout drawing headers
// ---------------------------------------------------------------------------

/// Harness layout drawing binary header.
///
/// **Container:** OLE2 compound document
pub const HARNESS_LAYOUT_DRAWING_BINARY_HEADER_V1: &str =
    "Altium Designer - Harness Layout Drawing Binary File Version 1.0";

/// Harness layout drawing ASCII header.
///
/// **Container:** plain text file
pub const HARNESS_LAYOUT_DRAWING_ASCII_HEADER_V1: &str =
    "Altium Designer - Harness Layout Drawing Ascii File Version 1.0";

/// Harness layout drawing JSON header.
///
/// **Container:** OLE2 compound document
pub const HARNESS_LAYOUT_DRAWING_JSON_HEADER_V1: &str =
    "Altium Designer - Harness Layout Drawing JSON File Version 1.0";

// ---------------------------------------------------------------------------
// Harness library headers
// ---------------------------------------------------------------------------

/// Harness library binary header.
///
/// **Container:** OLE2 compound document
pub const HARNESS_LIBRARY_BINARY_HEADER_V1: &str =
    "Altium Designer - Harness Library Binary File Version 1.0";

/// Harness library ASCII header.
///
/// **Container:** plain text file
pub const HARNESS_LIBRARY_ASCII_HEADER_V1: &str =
    "Altium Designer - Harness Library Ascii File Version 1.0";

/// Harness library JSON header.
///
/// **Container:** OLE2 compound document
pub const HARNESS_LIBRARY_JSON_HEADER_V1: &str =
    "Altium Designer - Harness Library JSON File Version 1.0";

// ---------------------------------------------------------------------------
// Electronics system design
// ---------------------------------------------------------------------------

/// Electronics system design JSON header.
///
/// **Container:** OLE2 compound document
pub const ELECTRONICS_SYSTEM_DESIGN_JSON_HEADER_V1: &str =
    "Altium Designer - Electronics System Design JSON File Version 1.0";

// ---------------------------------------------------------------------------
// PCB library headers
// ---------------------------------------------------------------------------

/// V6 binary PCB footprint library header (OLE2 compound document).
///
/// **Era:** Current (AD6+)
/// **Container:** OLE2 compound document
pub const PCB_LIBRARY_BINARY_HEADER_V6: &str = "PCB 6.0 Binary Library File";

/// Legacy binary PCB document header (`/FileHeader`, UTF-16LE).
///
/// **Container:** OLE2 compound document
pub const PCB_DOC_BINARY_HEADER_V5: &str = "PCB 5.0 Binary File";

/// V6 binary PCB document header (`/FileHeaderSix`, pascal-block).
///
/// **Container:** OLE2 compound document
pub const PCB_DOC_BINARY_HEADER_V6: &str = "PCB 6.0 Binary File";
