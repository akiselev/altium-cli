pub(crate) fn generate_unique_id() -> String {
    let hex = uuid::Uuid::new_v4().simple().to_string();
    hex[..8].to_ascii_uppercase()
}
