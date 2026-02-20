//! ID types for the v2 ID-handle architecture.
//!
//! These are slotmap keys used to identify records and groups within
//! a [`DocumentStore`](crate::store::DocumentStore).

use slotmap::new_key_type;

new_key_type! {
    /// Identifies a single record (RecordNode) within a DocumentStore.
    pub struct RecordId;
    /// Identifies a group (component or footprint) within a DocumentStore.
    pub struct GroupId;
}
