//! Sheet record (RECORD=31).

use crate::coord::SchCoord;
use altium_format_derive::altium_record;

/// Sheet record — document-level settings for a schematic sheet.
///
/// Corresponds to `SheetData` / `ExportSheet` in the v1 API (ObjectId::Sheet = 31).
///
/// Note: Font table entries (dynamic-count indexed fields like FontName1, Size1, etc.)
/// are skipped in this phase and will be handled in a later phase.
#[altium_record(kind = "sch", record_id = 31, codec = "params")]
pub struct SchSheetRecord {
    // --- Font table count (fonts themselves are skipped) ---
    #[altium(key = "FONTIDCOUNT")]
    font_id_count: i32,

    /// Font entries — skipped; handled in later phase.
    #[altium(skip)]
    _fonts: i32,

    // --- Document fields ---
    #[altium(key = "USEMBCS")]
    use_mbcs: bool,
    #[altium(key = "ISBOC")]
    is_boc: bool,
    #[altium(key = "HOTSPOTGRIDON")]
    hot_spot_grid_on: bool,
    #[altium(key = "HOTSPOTGRIDSIZE")]
    hot_spot_grid_size: SchCoord,
    #[altium(key = "SHEETSTYLE")]
    sheet_style: u8,
    #[altium(key = "SYSTEMFONT")]
    system_font: i32,
    #[altium(key = "DOCUMENTBORDERSTYLE")]
    document_border_style: u8,
    #[altium(key = "WORKSPACEORIENTATION")]
    workspace_orientation: u8,
    #[altium(key = "BORDERON")]
    border_on: bool,
    #[altium(key = "TITLEBLOCKON")]
    title_block_on: bool,
    #[altium(key = "SHEETNUMBERSPACESIZE")]
    sheet_number_space_size: i32,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "AREACOLOR")]
    area_color: u32,
    #[altium(key = "SNAPGRIDON")]
    snap_grid_on: bool,
    #[altium(key = "SNAPGRIDSIZE")]
    snap_grid_size: SchCoord,
    #[altium(key = "VISIBLEGRIDON")]
    visible_grid_on: bool,
    #[altium(key = "VISIBLEGRIDSIZE")]
    visible_grid_size: SchCoord,
    #[altium(key = "CUSTOMX")]
    custom_x: SchCoord,
    #[altium(key = "CUSTOMY")]
    custom_y: SchCoord,
    #[altium(key = "USECUSTOMSHEET")]
    use_custom_sheet: bool,
    #[altium(key = "SHOWHIDDENPINS")]
    show_hidden_pins: bool,
    #[altium(key = "REFERENCEZONESON")]
    reference_zones_on: bool,
    #[altium(key = "CUSTOMXZONES")]
    custom_x_zones: i32,
    #[altium(key = "CUSTOMYZONES")]
    custom_y_zones: i32,
    #[altium(key = "CUSTOMMARGINWIDTH")]
    custom_margin_width: SchCoord,
    #[altium(key = "SHOWTEMPLATEGRAPHICS")]
    show_template_graphics: bool,
    #[altium(key = "TEMPLATEFILENAME")]
    template_file_name: String,
    #[altium(key = "DISPLAY_UNIT")]
    display_unit: u8,
    #[altium(key = "REFERENCEZONESTYLE")]
    reference_zone_style: u8,
    #[altium(key = "ALWAYSSHOWCD")]
    always_show_cd: bool,

    // --- Vault/GUID fields ---
    #[altium(key = "RELEASEVAULTGUID")]
    release_vault_guid: String,
    #[altium(key = "RELEASEITEMGUID")]
    release_item_guid: String,
    #[altium(key = "ITEMREVISIONGUID")]
    item_revision_guid: String,
    #[altium(key = "PROPSVAULTGUID")]
    props_vault_guid: String,
    #[altium(key = "PROPSREVISIONGUID")]
    props_revision_guid: String,
    #[altium(key = "FILEVERSIONINFO")]
    file_version_info: String,
    #[altium(key = "TEMPLATEVAULTGUID")]
    template_vault_guid: String,
    #[altium(key = "TEMPLATEITEMGUID")]
    template_item_guid: String,
    #[altium(key = "TEMPLATEREVISIONGUID")]
    template_revision_guid: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=31|FONTIDCOUNT=2|USEMBCS=T|ISBOC=F|SHEETSTYLE=0|BORDERON=T|TITLEBLOCKON=T|COLOR=16777215|AREACOLOR=16777215|SNAPGRIDON=T|VISIBLEGRIDON=T|USECUSTOMSHEET=F|SHOWHIDDENPINS=F|REFERENCEZONESON=T|ALWAYSSHOWCD=F|",
        ));
        let rec = SchSheetRecord::from_origin(origin);
        assert_eq!(rec.font_id_count(), 2);
        assert!(rec.use_mbcs());
        assert!(rec.border_on());
        assert!(rec.title_block_on());
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=31|SHEETSTYLE=0|"));
        let mut rec = SchSheetRecord::from_origin(origin);
        rec.set_sheet_style(1);
        assert_eq!(rec.sheet_style(), 1);
        rec.set_border_on(false);
        assert!(!rec.border_on());
    }
}
