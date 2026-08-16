use std::ops::Range;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Spec-language domains understood by the language and synchronization layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecDomain {
    SchLib,
    PcbLib,
    SchDoc,
    PcbDoc,
    PrjPcb,
}

/// Stable-enough identity for a source node within one authored file revision.
///
/// Durable cross-revision identity lives in `altium-sync::BindingId`; this type is
/// intentionally source-local and never used as the document mutation address.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceNodeId(pub String);

/// A losslessly retained top-level resource block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBlock {
    pub id: SourceNodeId,
    pub kind: String,
    pub key: String,
    pub span: Range<usize>,
    pub source: String,
}

/// Lossless source container used by the synchronization layer.
///
/// The existing full parser remains authoritative for syntax and semantics while
/// it is migrated out of `altium-format-spec`. This type is deliberately a
/// structural scanner: it retains every byte and only identifies top-level
/// resource boundaries needed for source patches and three-way conflict scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LosslessSpec {
    source: String,
    resources: Vec<ResourceBlock>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SourceError {
    #[error("unterminated top-level block beginning at byte {0}")]
    UnterminatedBlock(usize),
}

impl LosslessSpec {
    pub fn parse(source: impl Into<String>) -> Result<Self, SourceError> {
        let source = source.into();
        let resources = scan_top_level_resources(&source)?;
        Ok(Self { source, resources })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn resources(&self) -> &[ResourceBlock] {
        &self.resources
    }

    pub fn into_source(self) -> String {
        self.source
    }
}

/// Normalize text for semantic fingerprints without changing authored contents.
///
/// Generated annotation IDs are management metadata, so they do not participate
/// in semantic equality. Line endings and trailing whitespace are normalized to
/// make snapshots independent of platform formatting.
pub fn canonicalize_semantic_text(source: &str) -> String {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::new();
    for line in normalized.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[annotation(") {
            continue;
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

fn scan_top_level_resources(source: &str) -> Result<Vec<ResourceBlock>, SourceError> {
    let mut resources = Vec::new();
    let mut offset = 0usize;
    let mut depth = 0i32;
    let mut block_start: Option<usize> = None;
    let mut block_kind = String::new();
    let mut block_key = String::new();
    let mut ordinal = 0usize;

    for line_with_newline in source.split_inclusive('\n') {
        let line = line_with_newline
            .strip_suffix('\n')
            .unwrap_or(line_with_newline);
        let trimmed = line.trim_start();

        if depth == 0 && block_start.is_none() {
            if let Some((kind, key)) = top_level_header(trimmed) {
                if line_contains_open_brace(trimmed) {
                    block_start = Some(offset);
                    block_kind = kind;
                    block_key = key;
                }
            }
        }

        let delta = brace_delta(line);
        depth += delta;

        if let Some(start) = block_start {
            if depth == 0 {
                let end = offset + line_with_newline.len();
                let source_block = source[start..end].to_string();
                resources.push(ResourceBlock {
                    id: SourceNodeId(format!("{}:{}:{}", block_kind, block_key, ordinal)),
                    kind: block_kind.clone(),
                    key: block_key.clone(),
                    span: start..end,
                    source: source_block,
                });
                ordinal += 1;
                block_start = None;
                block_kind.clear();
                block_key.clear();
            }
        }

        offset += line_with_newline.len();
    }

    // `split_inclusive` omits an empty final line, but all non-empty final lines
    // were processed above. A positive depth therefore means a real truncation.
    if let Some(start) = block_start {
        return Err(SourceError::UnterminatedBlock(start));
    }

    Ok(resources)
}

fn top_level_header(line: &str) -> Option<(String, String)> {
    const KINDS: &[&str] = &[
        "component",
        "footprint",
        "sheet",
        "board",
        "project",
        "placement",
    ];

    for kind in KINDS {
        if line == *kind
            || line.starts_with(&format!("{kind} "))
            || line.starts_with(&format!("{kind}{{"))
        {
            let rest = line[kind.len()..].trim_start();
            let key = parse_header_key(rest).unwrap_or_else(|| (*kind).to_string());
            return Some(((*kind).to_string(), key));
        }
    }
    None
}

fn parse_header_key(rest: &str) -> Option<String> {
    let rest = rest.trim_start();
    if rest.starts_with('{') || rest.is_empty() {
        return None;
    }
    if let Some(stripped) = rest.strip_prefix('"') {
        let mut escaped = false;
        let mut value = String::new();
        for ch in stripped.chars() {
            if escaped {
                value.push(ch);
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => return Some(value),
                _ => value.push(ch),
            }
        }
        return Some(value);
    }

    let token = rest
        .split(|ch: char| ch.is_whitespace() || ch == '{')
        .next()
        .unwrap_or("")
        .trim();
    (!token.is_empty()).then(|| token.to_string())
}

fn line_contains_open_brace(line: &str) -> bool {
    brace_delta(line) > 0 || line.chars().any(|ch| ch == '{')
}

fn brace_delta(line: &str) -> i32 {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut delta = 0i32;
    let mut in_string = false;
    let mut escaped = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if b == b'"' {
            in_string = true;
            i += 1;
            continue;
        }
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            break;
        }
        if b == b'{' {
            delta += 1;
        } else if b == b'}' {
            delta -= 1;
        }
        i += 1;
    }
    delta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_named_and_anonymous_resources_losslessly() {
        let source =
            "// lead\ncomponent \"U Part\" {\n  description: \"{literal}\"\n}\n\nboard {\n}\n";
        let spec = LosslessSpec::parse(source).unwrap();
        assert_eq!(spec.source(), source);
        assert_eq!(spec.resources().len(), 2);
        assert_eq!(spec.resources()[0].kind, "component");
        assert_eq!(spec.resources()[0].key, "U Part");
        assert_eq!(spec.resources()[1].key, "board");
    }

    #[test]
    fn canonicalization_ignores_management_annotations() {
        let a = "#[annotation(id = \"aaa\")]\ncomponent R {\n  x: 1   \n}\n";
        let b = "#[annotation(id = \"bbb\")]\r\ncomponent R {\r\n  x: 1\r\n}\r\n";
        assert_eq!(canonicalize_semantic_text(a), canonicalize_semantic_text(b));
    }

    #[test]
    fn reports_unterminated_resource() {
        let err = LosslessSpec::parse("component R {\n").unwrap_err();
        assert!(matches!(err, SourceError::UnterminatedBlock(0)));
    }
}
