//! Rebuild command.
//!
//! Rebuilds a supported Altium file from high-level typed APIs and prints a
//! stream-level CFB diff against the original.

use std::path::Path;

use serde::Serialize;

use crate::output::{self, TextFormat};

pub fn run(path: &Path, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let report = altium_format_ops::rebuild::cmd_rebuild(path)?;
    output::print(&TextWrapper(report), format)
}

#[derive(Serialize)]
#[serde(transparent)]
struct TextWrapper<T>(T);

impl<T: Serialize> TextFormat for TextWrapper<T> {
    fn format_text(&self) -> String {
        let value = match serde_json::to_value(&self.0) {
            Ok(v) => v,
            Err(e) => return format!("Failed to render rebuild report: {}", e),
        };

        let mut out = String::new();
        out.push_str("Rebuild Report\n");
        out.push_str("=============\n");

        if let Some(file_type) = value.get("file_type").and_then(|v| v.as_str()) {
            out.push_str(&format!("Type: {}\n", file_type));
        }
        if let Some(src) = value.get("source_path").and_then(|v| v.as_str()) {
            out.push_str(&format!("Source: {}\n", src));
        }
        if let Some(dst) = value.get("rebuilt_path").and_then(|v| v.as_str()) {
            out.push_str(&format!("Rebuilt: {}\n", dst));
        }

        if let Some(skipped) = value.get("skipped_records").and_then(|v| v.as_array()) {
            out.push_str("\nSkipped Records\n");
            out.push_str("---------------\n");
            if skipped.is_empty() {
                out.push_str("none\n");
            } else {
                for item in skipped {
                    let context = item.get("context").and_then(|v| v.as_str()).unwrap_or("?");
                    let rid = item.get("record_id").and_then(|v| v.as_u64()).unwrap_or(0);
                    let count = item.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                    let reason = item.get("reason").and_then(|v| v.as_str()).unwrap_or("");
                    out.push_str(&format!(
                        "context={} record_id={} count={} reason={}\n",
                        context, rid, count, reason
                    ));
                }
            }
        }

        if let Some(diff) = value.get("diff") {
            let matched = diff
                .get("matched")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let text_diffs = diff
                .get("text_diffs")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let binary_diffs = diff
                .get("binary_diffs")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let only_orig = diff
                .get("only_in_original")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let only_rebuilt = diff
                .get("only_in_rebuilt")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);

            out.push_str("\nDiff Summary\n");
            out.push_str("------------\n");
            out.push_str(&format!("matched_streams={}\n", matched));
            out.push_str(&format!("text_diffs={}\n", text_diffs));
            out.push_str(&format!("binary_diffs={}\n", binary_diffs));
            out.push_str(&format!("only_in_original={}\n", only_orig));
            out.push_str(&format!("only_in_rebuilt={}\n", only_rebuilt));

            if let Some(text_items) = diff.get("text_diffs").and_then(|v| v.as_array()) {
                if !text_items.is_empty() {
                    out.push_str("\nText Stream Diffs\n");
                    out.push_str("-----------------\n");
                    for item in text_items {
                        if let Some(name) = item.get("stream_name").and_then(|v| v.as_str()) {
                            out.push_str(&format!("{}\n", name));
                        }
                    }
                }
            }

            if let Some(bin_items) = diff.get("binary_diffs").and_then(|v| v.as_array()) {
                if !bin_items.is_empty() {
                    out.push_str("\nBinary Stream Diffs\n");
                    out.push_str("-------------------\n");
                    for item in bin_items {
                        if let Some(name) = item.get("stream_name").and_then(|v| v.as_str()) {
                            out.push_str(&format!("{}\n", name));
                        }
                    }
                }
            }
        }

        out
    }
}
