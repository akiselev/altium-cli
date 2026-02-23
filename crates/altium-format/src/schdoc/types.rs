use crate::sch_records::SchRecord;

#[derive(Debug)]
pub(crate) struct SchDocHeaderMetadata {
    pub header: String,
    pub weight: i32,
    pub minor_version: i32,
    pub unique_id: String,
}

#[derive(Debug)]
pub(crate) struct SchDocEmbeddedObject {
    pub id: String,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct SchDoc {
    pub(crate) header: SchDocHeaderMetadata,
    pub(crate) records: Vec<SchRecord>,
    pub(crate) additional_records: Vec<SchRecord>,
    pub(crate) embedded_objects: Vec<SchDocEmbeddedObject>,
}
