//! Template functions returning `RecordOrigin` with Altium-correct defaults.
//!
//! Each template creates a backing store pre-populated with the default
//! parameter values that Altium writes for new records.

use crate::v2::backing_store::{BinaryOrigin, ParamOrigin, RecordOrigin};

// ---------------------------------------------------------------------------
// Schematic templates (Track 6A)
// ---------------------------------------------------------------------------

pub fn sch_component_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=1|LIBREFERENCE=|COMPONENTDESCRIPTION=|PARTCOUNT=1|DISPLAYMODECOUNT=1|LOCATION.X=0|LOCATION.Y=0|CURRENTPARTID=1|LIBRARYPATH=|SOURCELIBRARYNAME=|TARGETFILENAME=|UNIQUEID=|AREACOLOR=11599871|COLOR=128|ORIENTATION=0|"
    ))
}

pub fn sch_pin_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=2|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|SYMBOLINNEREDGE=0|SYMBOLOUTEREDGE=0|SYMBOLINNER=0|SYMBOLOUTER=0|DESCRIPTION=|FORMALTYPE=1|ELECTRICAL=4|PINCONGLOMERATE=0|PINLENGTH=30|LOCATION.X=0|LOCATION.Y=0|NAME=|DESIGNATOR=|UNIQUEID=|"
    ))
}

pub fn sch_symbol_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=3|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|"
    ))
}

pub fn sch_label_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=4|OWNERINDEX=0|OWNERPARTID=1|LOCATION.X=0|LOCATION.Y=0|COLOR=8388608|FONTID=1|TEXT=|ORIENTATION=0|JUSTIFICATION=0|"
    ))
}

pub fn sch_bezier_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=5|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|COLOR=128|LINEWIDTH=1|LOCATIONCOUNT=0|"
    ))
}

pub fn sch_polyline_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=6|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|COLOR=128|LINEWIDTH=1|LOCATIONCOUNT=0|"
    ))
}

pub fn sch_polygon_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=7|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|COLOR=128|AREACOLOR=16777215|ISSOLID=T|LINEWIDTH=1|LOCATIONCOUNT=0|"
    ))
}

pub fn sch_ellipse_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=8|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|LOCATION.X=0|LOCATION.Y=0|RADIUS=10|RADIUSX=10|RADIUSY=10|COLOR=128|AREACOLOR=16777215|ISSOLID=T|LINEWIDTH=1|"
    ))
}

pub fn sch_pie_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=9|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|LOCATION.X=0|LOCATION.Y=0|RADIUS=10|STARTANGLE=0|ENDANGLE=360|COLOR=128|AREACOLOR=16777215|ISSOLID=T|LINEWIDTH=1|"
    ))
}

pub fn sch_round_rectangle_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=10|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|LOCATION.X=0|LOCATION.Y=0|CORNER.X=10|CORNER.Y=10|CORNERXRADIUS=0|CORNERYRADIUS=0|COLOR=128|AREACOLOR=16777215|ISSOLID=T|LINEWIDTH=1|"
    ))
}

pub fn sch_elliptical_arc_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=11|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|LOCATION.X=0|LOCATION.Y=0|RADIUS=10|SECONDARYRADIUS=10|STARTANGLE=0|ENDANGLE=360|COLOR=128|LINEWIDTH=1|"
    ))
}

pub fn sch_arc_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=12|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|LOCATION.X=0|LOCATION.Y=0|RADIUS=10|STARTANGLE=0|ENDANGLE=360|COLOR=128|LINEWIDTH=1|"
    ))
}

pub fn sch_line_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=13|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|LOCATION.X=0|LOCATION.Y=0|CORNER.X=10|CORNER.Y=10|COLOR=128|LINEWIDTH=1|"
    ))
}

pub fn sch_rectangle_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=14|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|LOCATION.X=0|LOCATION.Y=0|CORNER.X=10|CORNER.Y=10|COLOR=128|AREACOLOR=16777215|ISSOLID=T|LINEWIDTH=1|TRANSPARENT=T|"
    ))
}

pub fn sch_sheet_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=31|FONTIDCOUNT=0|AREACOLOR=16317695|BORDERON=T|TITLEBLOCKON=T|SNAPGRIDON=T|SNAPGRIDSIZE=10|VISIBLEGRIDON=T|VISIBLEGRIDSIZE=10|CUSTOMX=1100|CUSTOMY=950|USECUSTOMSHEET=F|WORKSPACEORIENTATION=1|"
    ))
}

// Additional schematic record defaults

pub fn sch_power_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=17|OWNERPARTID=-1|STYLE=1|SHOWNETNAME=T|LOCATION.X=0|LOCATION.Y=0|ORIENTATION=0|COLOR=128|TEXT=|UNIQUEID=|"
    ))
}

pub fn sch_port_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=18|OWNERPARTID=-1|STYLE=3|IOTYPE=3|ALIGNMENT=0|WIDTH=40|LOCATION.X=0|LOCATION.Y=0|AREACOLOR=16777215|COLOR=128|TEXTCOLOR=128|NAME=|UNIQUEID=|"
    ))
}

pub fn sch_no_erc_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=22|OWNERINDEX=0|OWNERPARTID=-1|LOCATION.X=0|LOCATION.Y=0|COLOR=128|ORIENTATION=0|"
    ))
}

pub fn sch_net_label_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=25|OWNERPARTID=-1|LOCATION.X=0|LOCATION.Y=0|COLOR=8388608|FONTID=1|TEXT=|ORIENTATION=0|"
    ))
}

pub fn sch_bus_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=26|OWNERPARTID=-1|LINEWIDTH=1|COLOR=128|LOCATIONCOUNT=2|X1=0|Y1=0|X2=10|Y2=0|"
    ))
}

pub fn sch_wire_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=27|OWNERPARTID=-1|LINEWIDTH=1|COLOR=128|LOCATIONCOUNT=2|X1=0|Y1=0|X2=10|Y2=0|UNIQUEID=|"
    ))
}

pub fn sch_text_frame_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=28|OWNERPARTID=-1|LOCATION.X=0|LOCATION.Y=0|CORNER.X=10|CORNER.Y=10|COLOR=128|AREACOLOR=16777215|TEXTCOLOR=128|FONTID=1|ALIGNMENT=1|WORDWRAP=T|TEXT=|CLIPTORECT=T|"
    ))
}

pub fn sch_junction_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=29|OWNERPARTID=-1|LOCATION.X=0|LOCATION.Y=0|COLOR=128|"
    ))
}

pub fn sch_image_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=30|OWNERINDEX=0|OWNERPARTID=1|OWNERPARTDISPLAYMODE=0|LOCATION.X=0|LOCATION.Y=0|CORNER.X=10|CORNER.Y=10|KEEPASPECT=T|EMBEDIMAGE=T|FILENAME=|"
    ))
}

pub fn sch_designator_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=34|OWNERINDEX=0|OWNERPARTID=1|LOCATION.X=0|LOCATION.Y=0|COLOR=8388608|FONTID=1|TEXT=|NAME=Designator|READONLYSTATE=1|UNIQUEID=|"
    ))
}

pub fn sch_parameter_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=41|OWNERINDEX=0|OWNERPARTID=1|LOCATION.X=0|LOCATION.Y=0|COLOR=8388608|FONTID=1|TEXT=|NAME=Value|READONLYSTATE=1|UNIQUEID=|"
    ))
}

pub fn sch_bus_entry_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=37|OWNERPARTID=-1|LOCATION.X=0|LOCATION.Y=0|CORNER.X=5|CORNER.Y=5|COLOR=128|LINEWIDTH=1|"
    ))
}

pub fn sch_sheet_symbol_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=39|OWNERPARTID=-1|LOCATION.X=0|LOCATION.Y=0|XSIZE=100|YSIZE=100|COLOR=128|AREACOLOR=16777215|ISSOLID=T|UNIQUEID=|"
    ))
}

pub fn sch_sheet_entry_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=40|OWNERINDEX=0|OWNERPARTID=-1|STYLE=3|IOTYPE=3|SIDE=0|DISTANCEFROMTOP=0|COLOR=128|AREACOLOR=8388608|TEXTCOLOR=128|TEXTFONTID=1|ARROWKIND=0|NAME=|UNIQUEID=|"
    ))
}

pub fn sch_sheet_name_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=32|OWNERINDEX=0|OWNERPARTID=-1|LOCATION.X=0|LOCATION.Y=0|COLOR=128|FONTID=1|TEXT=|"
    ))
}

pub fn sch_sheet_filename_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=33|OWNERINDEX=0|OWNERPARTID=-1|LOCATION.X=0|LOCATION.Y=0|COLOR=128|FONTID=1|TEXT=|"
    ))
}

pub fn sch_implementation_list_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new("|RECORD=44|OWNERINDEX=0|"))
}

pub fn sch_implementation_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=45|OWNERINDEX=0|MODELNAME=|MODELTYPE=|DATAFILECOUNT=0|MODELDATAFILEENTITY0=|MODELDATAFILEKIND0=|ISCURRENT=T|DATALINKSLOCKED=T|DATABASEDATALINKSLOCKED=T|INTEGRATEDMODEL=T|DATABASEMODEL=T|"
    ))
}

pub fn sch_note_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=209|OWNERINDEX=0|OWNERPARTID=-1|LOCATION.X=0|LOCATION.Y=0|COLOR=128|FONTID=1|TEXT=|AUTHOR=|"
    ))
}

pub fn sch_blanket_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new(
        "|RECORD=255|OWNERINDEX=0|OWNERPARTID=-1|LOCATION.X=0|LOCATION.Y=0|CORNER.X=10|CORNER.Y=10|COLOR=128|LINEWIDTH=1|"
    ))
}

// ---------------------------------------------------------------------------
// PCB templates (Track 6B)
// ---------------------------------------------------------------------------

pub fn pcb_track_default() -> RecordOrigin {
    // 13-byte common header + 22 track-specific bytes (start_x, start_y, end_x, end_y, width, subpoly_index)
    let mut data = vec![0u8; 35];
    // Common header: net=0xFFFF (no net), polygon_ref=0x0000, component_ref=0xFFFF
    data[3] = 0xFF;
    data[4] = 0xFF; // net
    data[7] = 0xFF;
    data[8] = 0xFF; // component_ref
    data[9] = 0xFF;
    data[10] = 0xFF; // ref4
    data[11] = 0xFF;
    data[12] = 0xFF; // ref5
    RecordOrigin::Binary(BinaryOrigin::new(data))
}

pub fn pcb_arc_default() -> RecordOrigin {
    // 13-byte common header + 24 arc-specific bytes
    let mut data = vec![0u8; 37];
    data[3] = 0xFF;
    data[4] = 0xFF; // net
    data[7] = 0xFF;
    data[8] = 0xFF; // component_ref
    data[9] = 0xFF;
    data[10] = 0xFF;
    data[11] = 0xFF;
    data[12] = 0xFF;
    RecordOrigin::Binary(BinaryOrigin::new(data))
}

pub fn pcb_fill_default() -> RecordOrigin {
    // 13-byte common header + 20 fill-specific bytes
    let mut data = vec![0u8; 33];
    data[3] = 0xFF;
    data[4] = 0xFF;
    data[7] = 0xFF;
    data[8] = 0xFF;
    data[9] = 0xFF;
    data[10] = 0xFF;
    data[11] = 0xFF;
    data[12] = 0xFF;
    RecordOrigin::Binary(BinaryOrigin::new(data))
}

pub fn pcb_pad_default() -> RecordOrigin {
    // Complex record - just create a minimal binary block
    RecordOrigin::Binary(BinaryOrigin::new(vec![0u8; 64]))
}

pub fn pcb_via_default() -> RecordOrigin {
    RecordOrigin::Binary(BinaryOrigin::new(vec![0u8; 32]))
}

pub fn pcb_text_default() -> RecordOrigin {
    RecordOrigin::Binary(BinaryOrigin::new(vec![0u8; 32]))
}

pub fn pcb_region_default() -> RecordOrigin {
    RecordOrigin::Binary(BinaryOrigin::new(vec![0u8; 32]))
}

pub fn pcb_component_body_default() -> RecordOrigin {
    RecordOrigin::Binary(BinaryOrigin::new(vec![0u8; 32]))
}

pub fn pcb_footprint_default() -> RecordOrigin {
    RecordOrigin::Param(ParamOrigin::new("|PATTERN=|DESCRIPTION=|HEIGHT=0|"))
}
