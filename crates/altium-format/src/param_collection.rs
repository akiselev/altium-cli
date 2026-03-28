//! Layer 4 parameter collection for text-format Altium blocks.
//! Pipe-delimited key=value pairs decoded from Windows-1252 bytes.
//! Keys stored in original case; lookups are case-insensitive.
//! Accessors are destructive (remove-on-read): `assert_exhausted` then
//! confirms every key was consumed, enforcing the fail-fast invariant.
//! Insertion order is preserved (IndexMap) for deterministic serialization.
use altium_format_types::constants::parsing::{
    C_BASE_UNIT, C_SCH_BROKEN_BAR, C_SCH_SPECIAL_DELIMITER, C_SCH_UTF8_PREFIX,
};
use altium_format_types::constants::record_structure::{EX, EY};
use altium_format_types::constants::visual::EXTRA_LOCATION_COUNT;
use altium_format_types::{Coord, CoordPoint};
use indexmap::IndexMap;

use crate::param_value::{FromParamValue, ToParamValue};
use crate::{AltiumFormatError, Result};

#[derive(Clone)]
pub(crate) struct ParameterCollection {
    // Keys stored in original case for round-trip fidelity; lookups are case-insensitive.
    // IndexMap preserves insertion order for deterministic serialization.
    params: IndexMap<String, String>,
}

impl ParameterCollection {
    // Creates an empty collection; use from_bytes or from_utf16le_bytes to populate.
    pub(crate) fn new() -> Self {
        Self {
            params: IndexMap::new(),
        }
    }

    // Returns true if the collection has no parameters.
    pub(crate) fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// Returns the value for a key (case-insensitive lookup), or None if not present.
    pub(crate) fn get(&self, key: &str) -> Option<&str> {
        let key_upper = key.to_ascii_uppercase();
        self.params
            .iter()
            .find(|(k, _)| k.to_ascii_uppercase() == key_upper)
            .map(|(_, v)| v.as_str())
    }

    /// Iterates over all (key, value) pairs in insertion order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.params.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    // Inserts a key=value pair. Does nothing if the key already exists (first-occurrence-wins).
    pub(crate) fn insert(&mut self, key: &str, value: String) {
        self.params.entry(key.to_owned()).or_insert(value);
    }

    /// Inserts a key=value pair, overwriting any existing value for this key.
    pub(crate) fn set(&mut self, key: &str, value: String) {
        // Remove old entry (case-insensitive) then insert new.
        let key_upper = key.to_ascii_uppercase();
        self.params
            .retain(|k, _| k.to_ascii_uppercase() != key_upper);
        self.params.insert(key.to_owned(), value);
    }

    // Inserts a DXP fractional coordinate. Always writes the integer part; writes
    // the _FRAC companion only if the fractional remainder is non-zero.
    pub(crate) fn insert_coord(&mut self, key: &str, frac_key: &str, coord: Coord) {
        let internal = coord.to_internal();
        // Truncation division (toward zero) matches Altium's integer/frac split:
        // -1920000 → integer=-19, frac=-20000 (not Euclidean's -20, +80000).
        let integer_part = internal / C_BASE_UNIT;
        let frac_part = internal % C_BASE_UNIT;
        // T1 behavior: skip integer/frac parts when they are zero.
        if integer_part != 0 {
            self.insert(key, integer_part.to_param_value());
        }
        if frac_part != 0 {
            self.insert(frac_key, frac_part.to_param_value());
        }
    }

    // Parses DistanceFromTop which uses a non-standard fractional encoding.
    // Divisor is 1_000_000 (10x standard DXP), with legacy "_Frac" and new "_Frac1" variants.
    // See C# SchDataSerializer.ImportDistanceFromTop / ExportDistanceFromTop.
    pub(crate) fn remove_distance_from_top(&mut self, key: &str) -> Result<Coord> {
        let whole: i16 = self.remove_with_default(key, 0i16)?;
        let frac_key = format!("{key}_Frac");
        let frac1_key = format!("{key}_Frac1");
        let frac: i32 = self.remove_with_default(&frac_key, 0i32)?;
        let frac1: i32 = self.remove_with_default(&frac1_key, 0i32)?;

        let coord = if frac != 0 {
            // Legacy format: (whole * 100_000 + frac) * 10
            ((whole as i32) * 100_000 + frac) * 10
        } else if frac1 != 0 {
            // New format: whole * 1_000_000 + frac1
            (whole as i32) * 1_000_000 + frac1
        } else {
            // No frac: whole * 1_000_000
            (whole as i32) * 1_000_000
        };
        Ok(Coord::from_internal(coord))
    }

    // Serializes DistanceFromTop using new _Frac1 format (matching C# ExportDistanceFromTop).
    pub(crate) fn insert_distance_from_top(&mut self, key: &str, coord: Coord) {
        let val = coord.to_internal();
        let whole = val / 1_000_000;
        let frac1 = val - whole * 1_000_000;
        self.insert(key, (whole as i16).to_param_value());
        if frac1 != 0 {
            let frac1_key = format!("{key}_Frac1");
            self.insert(&frac1_key, frac1.to_param_value());
        }
    }

    // Inserts a coordinate point (x + y, each with optional frac).
    pub(crate) fn insert_coord_point(
        &mut self,
        x_key: &str,
        x_frac: &str,
        y_key: &str,
        y_frac: &str,
        point: CoordPoint,
    ) {
        self.insert_coord(x_key, x_frac, point.x);
        self.insert_coord(y_key, y_frac, point.y);
    }

    // Inserts indexed coordinates with Altium's 50-vertex split.
    // First 50 vertices: count_key=min(N,50), {x_prefix}1..50, {y_prefix}1..50.
    // Overflow (>50): EXTRALOCATIONCOUNT=N-50, EX51..EX{N}, EY51..EY{N}.
    // Uses T1 logic: each individual X/Y is skipped if the coordinate is zero.
    pub(crate) fn insert_indexed_coords(
        &mut self,
        count_key: &str,
        x_prefix: &str,
        y_prefix: &str,
        points: &[CoordPoint],
    ) {
        let base_count = points.len().min(50);
        let extra_count = points.len().saturating_sub(50);

        self.insert(count_key, base_count.to_param_value());

        // Base vertices: {x_prefix}1..{base_count}, {y_prefix}1..{base_count}
        for (i, point) in points[..base_count].iter().enumerate() {
            let idx = i + 1; // 1-based
            if point.x.to_internal() != 0 {
                let x_key = format!("{x_prefix}{idx}");
                let x_frac_key = format!("{x_prefix}{idx}_Frac");
                self.insert_coord(&x_key, &x_frac_key, point.x);
            }
            if point.y.to_internal() != 0 {
                let y_key = format!("{y_prefix}{idx}");
                let y_frac_key = format!("{y_prefix}{idx}_Frac");
                self.insert_coord(&y_key, &y_frac_key, point.y);
            }
        }

        // Extra vertices (>50): EX51..EX{N}, EY51..EY{N}
        if extra_count > 0 {
            self.insert(EXTRA_LOCATION_COUNT, extra_count.to_param_value());
            for (i, point) in points[base_count..].iter().enumerate() {
                let idx = base_count + i + 1; // continues from 51
                if point.x.to_internal() != 0 {
                    let x_key = format!("{EX}{idx}");
                    let x_frac_key = format!("{EX}{idx}_Frac");
                    self.insert_coord(&x_key, &x_frac_key, point.x);
                }
                if point.y.to_internal() != 0 {
                    let y_key = format!("{EY}{idx}");
                    let y_frac_key = format!("{EY}{idx}_Frac");
                    self.insert_coord(&y_key, &y_frac_key, point.y);
                }
            }
        }
    }

    // Serializes to pipe-delimited Windows-1252 bytes with %UTF8% dual-write.
    //
    // For each parameter, prepares a "safe" value (pipes → broken bar ¦) matching
    // C#'s `GetSafeParamValue`. If the safe value contains any char > '~' (the
    // `HasNonAnsiSymbols` trigger), a UTF-8 version `|%UTF8%KEY=VALUE||` is emitted
    // first. The Win-1252 version `|KEY=VALUE` is ALWAYS emitted (unconditionally).
    //
    // Pipe escaping in Win-1252 uses byte 0x8E (Ž in Win-1252): | → Ž, literal Ž → ŽŽ.
    // Equals signs are never escaped (parser splits on first `=` only).
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (key, value) in &self.params {
            // Prepare "safe" value: double Ž first, then pipes → broken bar.
            // Matches C#'s GetSafeParamValue: | → ¦, \u008E → \u008E\u008E.
            // Our in-memory Ž is U+017D (Win-1252 decoded form of byte 0x8E).
            let safe_value: String = {
                let s = value.replace('\u{017D}', "\u{017D}\u{017D}");
                s.replace('|', &String::from(C_SCH_BROKEN_BAR))
            };

            // HasNonAnsiSymbols: any char > '~' except the delimiter itself.
            let needs_utf8 = safe_value
                .chars()
                .any(|c| c > '~' && c != C_SCH_SPECIAL_DELIMITER);

            // If non-ASCII: emit UTF-8 version first (value is .trim()'d).
            if needs_utf8 {
                let trimmed = safe_value.trim();
                let utf8_entry = format!("|{}{key}={trimmed}||", C_SCH_UTF8_PREFIX);
                out.extend_from_slice(utf8_entry.as_bytes());
            }

            // ALWAYS emit Win-1252 version.
            let escaped = escape_for_win1252(value);
            out.push(b'|');
            let (encoded_key, _, _) = encoding_rs::WINDOWS_1252.encode(key);
            out.extend_from_slice(&encoded_key);
            out.push(b'=');
            let (encoded_value, _, _) = encoding_rs::WINDOWS_1252.encode(&escaped);
            out.extend_from_slice(&encoded_value);

            // Per-parameter 0x8E boundary guard: if encoded value ends with 0x8E,
            // append a trailing pipe to prevent misparse at the segment boundary.
            if encoded_value.last() == Some(&0x8E) {
                out.push(b'|');
            }
        }
        out.push(0); // NUL terminator
        out
    }

    // Serializes as pipe-delimited UTF-16LE parameter bytes (no NUL terminator).
    // Format: "|KEY1=VALUE1|KEY2=VALUE2|" encoded as UTF-16LE.
    // Pipes in values are escaped as ¦ (broken bar, U+00A6). Equals are not escaped.
    // Used for pin sidecar streams (PinMiscData, PinWideText, etc.).
    pub(crate) fn to_utf16le_bytes(&self) -> Vec<u8> {
        let mut s = String::new();
        for (key, value) in &self.params {
            let escaped_value = escape_for_utf16le(value);
            s.push('|');
            s.push_str(key);
            s.push('=');
            s.push_str(&escaped_value);
        }
        // No trailing pipe -- matches C#'s StrUtils.SetParameterValue output.
        // encoding_rs::UTF_16LE.encode() does NOT produce UTF-16LE (the WHATWG spec
        // has no encoder for UTF-16); use Rust's native encode_utf16() instead.
        s.encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect::<Vec<u8>>()
    }

    // Parses pipe-delimited Windows-1252 parameter bytes with %UTF8% key support.
    // Splitting on raw bytes before decoding preserves %UTF8% value integrity.
    pub(crate) fn from_bytes(data: &[u8]) -> Result<Self> {
        let data = data.strip_suffix(b"\0").unwrap_or(data);
        let mut params = IndexMap::new();
        for segment in data.split(|&b| b == b'|') {
            if segment.is_empty() {
                continue;
            }
            let eq_pos = match segment.iter().position(|&b| b == b'=') {
                Some(p) => p,
                None => {
                    let (key_str, _) =
                        encoding_rs::WINDOWS_1252.decode_without_bom_handling(segment);
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: key_str.into_owned(),
                        detail: "segment has no '=' separator".to_owned(),
                    });
                }
            };
            let raw_key = &segment[..eq_pos];
            let raw_value = &segment[eq_pos + 1..];
            let (key_str, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(raw_key);
            let key_str = key_str.into_owned();
            let value_str = if key_str.starts_with(C_SCH_UTF8_PREFIX) {
                let stripped_key = key_str[C_SCH_UTF8_PREFIX.len()..].to_owned();
                let value = std::str::from_utf8(raw_value).map_err(|e| {
                    AltiumFormatError::InvalidParamValue {
                        key: stripped_key.clone(),
                        detail: format!("UTF-8 decode error: {e}"),
                    }
                })?;
                let unescaped = unescape_param_value(value);
                params.entry(stripped_key).or_insert(unescaped);
                continue;
            } else {
                let (decoded, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(raw_value);
                unescape_param_value(&decoded)
            };
            // First occurrence wins for duplicate keys.
            params.entry(key_str).or_insert(value_str);
        }
        Ok(Self { params })
    }

    // Decodes UTF-16LE to &str then parses via from_str_params directly.
    // Re-encoding to bytes then decoding via Windows-1252 would corrupt non-ASCII characters.
    pub(crate) fn from_utf16le_bytes(data: &[u8]) -> Result<Self> {
        let (decoded, had_errors) = encoding_rs::UTF_16LE.decode_without_bom_handling(data);
        if had_errors {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "UTF-16LE".to_owned(),
                detail: "invalid UTF-16LE encoding in parameter data".to_owned(),
            });
        }
        Self::from_str_params(&decoded)
    }

    // Treats all values as already-decoded strings; %UTF8% key prefix handling
    // does not apply here. Only from_bytes (raw-byte path) strips %UTF8% and
    // switches to UTF-8 decoding for the value bytes.
    pub(crate) fn from_str(s: &str) -> Result<Self> {
        Self::from_str_params(s)
    }

    fn from_str_params(s: &str) -> Result<Self> {
        let s = s.strip_suffix('\0').unwrap_or(s);
        let mut params = IndexMap::new();
        for segment in s.split('|') {
            if segment.is_empty() {
                continue;
            }
            let eq_pos = match segment.find('=') {
                Some(p) => p,
                None => {
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: segment.to_owned(),
                        detail: "segment has no '=' separator".to_owned(),
                    });
                }
            };
            let key = &segment[..eq_pos];
            let value = unescape_param_value(&segment[eq_pos + 1..]);
            params.entry(key.to_owned()).or_insert(value);
        }
        Ok(Self { params })
    }

    // Removes key (case-insensitive), parses value via FromParamValue. Errors if absent.
    // shift_remove preserves insertion order for the remaining keys (swap_remove would not).
    pub(crate) fn remove_required<T: FromParamValue>(&mut self, key: &str) -> Result<T> {
        let found = self.find_key(key).map(|k| k.to_owned());
        match found {
            Some(actual_key) => {
                let value = self
                    .params
                    .shift_remove(&actual_key)
                    .expect("key found by find_key");
                T::from_param_value(&actual_key, &value)
            }
            None => Err(AltiumFormatError::MissingParam(key.to_owned())),
        }
    }

    // Removes key (case-insensitive) and parses it if present; returns Ok(None) if absent.
    pub(crate) fn remove_optional<T: FromParamValue>(&mut self, key: &str) -> Result<Option<T>> {
        let found = self.find_key(key).map(|k| k.to_owned());
        match found {
            Some(actual_key) => {
                let value = self
                    .params
                    .shift_remove(&actual_key)
                    .expect("key found by find_key");
                T::from_param_value(&actual_key, &value).map(Some)
            }
            None => Ok(None),
        }
    }

    // Removes and parses the key if present; returns `default` when the key is absent.
    pub(crate) fn remove_with_default<T: FromParamValue>(
        &mut self,
        key: &str,
        default: T,
    ) -> Result<T> {
        match self.remove_optional::<T>(key)? {
            Some(v) => Ok(v),
            None => Ok(default),
        }
    }

    // Reconstructs a Coord from integer + fractional DXP parts: N * 100_000 + F.
    pub(crate) fn remove_coord(&mut self, key: &str, frac_key: &str) -> Result<Coord> {
        let integer: i32 = self.remove_with_default(key, 0i32)?;
        let frac: i32 = self.remove_with_default(frac_key, 0i32)?;
        Ok(Coord::from_dxp_frac(integer, frac))
    }

    // Like remove_coord, but returns None if the integer part is absent.
    // Still consumes the frac companion if present (to avoid assert_exhausted failures).
    pub(crate) fn remove_coord_optional(
        &mut self,
        key: &str,
        frac_key: &str,
    ) -> Result<Option<Coord>> {
        let integer: Option<i32> = self.remove_optional(key)?;
        let frac: Option<i32> = self.remove_optional::<i32>(frac_key)?;
        match (integer, frac) {
            (Some(int_val), frac_val) => {
                Ok(Some(Coord::from_dxp_frac(int_val, frac_val.unwrap_or(0))))
            }
            // Some SchDoc fields may be serialized with only *_FRAC present; preserve them as 0+frac.
            (None, Some(frac_val)) => Ok(Some(Coord::from_dxp_frac(0, frac_val))),
            (None, None) => Ok(None),
        }
    }

    // Reads count from `count_key`, then removes `{x_prefix}N`/`{y_prefix}N` pairs as Coords.
    // Indices are 1-based to match Altium's on-disk format (X1, Y1, X2, Y2, ...).
    // When vertex count > 50, Altium splits storage: first 50 use X/Y prefix,
    // overflow uses EXTRALOCATIONCOUNT + EX/EY prefix with indices continuing from 51.
    pub(crate) fn remove_indexed_coords(
        &mut self,
        count_key: &str,
        x_prefix: &str,
        y_prefix: &str,
    ) -> Result<Vec<CoordPoint>> {
        let base_count: usize = self.remove_required(count_key)?;
        let extra_count: usize = self
            .remove_optional::<usize>(EXTRA_LOCATION_COUNT)?
            .unwrap_or(0);
        let total = base_count + extra_count;
        let mut points = Vec::with_capacity(total);

        // Base vertices: {x_prefix}1..{base_count}, {y_prefix}1..{base_count}
        for i in 1..=base_count {
            let x_key = format!("{x_prefix}{i}");
            let y_key = format!("{y_prefix}{i}");
            let x_frac_key = format!("{x_prefix}{i}_Frac");
            let y_frac_key = format!("{y_prefix}{i}_Frac");
            let x = self.remove_coord(&x_key, &x_frac_key)?;
            let y = self.remove_coord(&y_key, &y_frac_key)?;
            points.push(CoordPoint::new(x, y));
        }

        // Extra vertices: EX{base_count+1}..EX{total}, EY{base_count+1}..EY{total}
        // Always uses hardcoded EX/EY prefix per C# SchDataVertices.ExportToFile.
        for i in (base_count + 1)..=(base_count + extra_count) {
            let x_key = format!("{EX}{i}");
            let y_key = format!("{EY}{i}");
            let x_frac_key = format!("{EX}{i}_Frac");
            let y_frac_key = format!("{EY}{i}_Frac");
            let x = self.remove_coord(&x_key, &x_frac_key)?;
            let y = self.remove_coord(&y_key, &y_frac_key)?;
            points.push(CoordPoint::new(x, y));
        }

        Ok(points)
    }

    // Reads count from `count_key`, then calls `parse_one(self, i)` for i in base..base+count.
    pub(crate) fn remove_indexed<T>(
        &mut self,
        count_key: &str,
        base: usize,
        mut parse_one: impl FnMut(&mut Self, usize) -> Result<T>,
    ) -> Result<Vec<T>> {
        let count: usize = self.remove_required(count_key)?;
        let mut items = Vec::with_capacity(count);
        for i in base..base + count {
            items.push(parse_one(self, i)?);
        }
        Ok(items)
    }

    // Removes a comma-separated value and parses each element; errors if key absent.
    pub(crate) fn remove_list<T: FromParamValue>(&mut self, key: &str) -> Result<Vec<T>> {
        let raw: String = self.remove_required(key)?;
        raw.split(',')
            .map(|s| T::from_param_value(key, s.trim()))
            .collect()
    }

    // Like remove_list but returns empty Vec when the key is absent.
    pub(crate) fn remove_list_or_empty<T: FromParamValue>(&mut self, key: &str) -> Result<Vec<T>> {
        match self.remove_optional::<String>(key)? {
            Some(raw) => raw
                .split(',')
                .map(|s| T::from_param_value(key, s.trim()))
                .collect(),
            None => Ok(vec![]),
        }
    }

    // Returns the keys that have not yet been consumed; used for debugging.
    pub(crate) fn remaining_keys(&self) -> Vec<&str> {
        self.params.keys().map(String::as_str).collect()
    }

    // Returns the count of unconsumed keys.
    pub(crate) fn remaining_count(&self) -> usize {
        self.params.len()
    }

    // Drains all remaining key-value pairs, returning them as a Vec.
    // Used for generic/partial parsing where remaining params are captured
    // but not yet type-checked (e.g. transitional violation/options parsing).
    pub(crate) fn drain_remaining(&mut self) -> Vec<(String, String)> {
        self.params.drain(..).collect()
    }

    // Returns Err(UnknownParams) if any keys remain unconsumed.
    // Call at the dispatch boundary after all known fields are removed.
    pub(crate) fn assert_exhausted(&self) -> Result<()> {
        if self.params.is_empty() {
            return Ok(());
        }
        let keys: Vec<String> = self.params.keys().cloned().collect();
        Err(AltiumFormatError::UnknownParams { keys })
    }

    /// Returns all keys whose names start with `prefix` (case-insensitive).
    pub(crate) fn keys_matching(&self, prefix: &str) -> Vec<String> {
        let lower_prefix = prefix.to_ascii_lowercase();
        self.params
            .keys()
            .filter(|k| k.to_ascii_lowercase().starts_with(&lower_prefix))
            .cloned()
            .collect()
    }

    /// Removes and returns all key-value pairs whose key starts with `prefix`
    /// (case-insensitive). Used for consuming families of indexed parameters
    /// (e.g. all `V9_CACHE_LAYER{N}_*` entries) as structured data.
    ///
    /// Unlike drain_remaining, this targets explicit known prefixes and preserves
    /// the data. Unknown keys (not matching any prefix) will still trigger
    /// assert_exhausted errors.
    pub(crate) fn remove_prefixed(&mut self, prefix: &str) -> IndexMap<String, String> {
        let keys = self.keys_matching(prefix);
        let mut result = IndexMap::new();
        for key in keys {
            if let Some(value) = self.params.shift_remove(&key) {
                result.insert(key, value);
            }
        }
        result
    }

    /// Applies the Altium UNICODE sidecar mechanism to this collection.
    ///
    /// Altium uses a sidecar convention for fields that contain characters outside
    /// Windows-1252: `UNICODE=EXISTS` is a marker flag, and `UNICODE__<KEY>` provides
    /// the true Unicode value as comma-separated decimal UTF-16 code points.
    ///
    /// This method:
    /// 1. Removes the `UNICODE` marker flag (if present; returns Ok(()) if absent)
    /// 2. Removes all `UNICODE__*` sidecar keys
    /// 3. Decodes each sidecar value from UTF-16 code points to a Rust String
    /// 4. Replaces the corresponding parameter (`<KEY>`) with the decoded Unicode value
    ///
    /// Must be called BEFORE extracting field values so that callers receive the
    /// correct Unicode text instead of garbled Windows-1252 bytes.
    pub(crate) fn apply_unicode_sidecars(&mut self) -> Result<()> {
        // If no UNICODE marker, nothing to do.
        if self.remove_optional::<String>("UNICODE")?.is_none() {
            return Ok(());
        }

        let sidecars = self.remove_prefixed("UNICODE__");
        for (sidecar_key, encoded_value) in sidecars {
            // Strip the "UNICODE__" prefix (case-insensitive) to get the target field name.
            let lower = sidecar_key.to_ascii_lowercase();
            let field_name = lower.strip_prefix("unicode__").ok_or_else(|| {
                AltiumFormatError::InvalidParamValue {
                    key: sidecar_key.clone(),
                    detail: "UNICODE sidecar key missing expected prefix".to_owned(),
                }
            })?;

            // Decode comma-separated decimal UTF-16 code points.
            let decoded = decode_unicode_sidecar(&sidecar_key, &encoded_value)?;

            // Replace the target parameter with the decoded Unicode value.
            // The target key may have different casing in the map, so find it.
            if let Some(actual_key) = self.find_key(field_name).map(|s| s.to_owned()) {
                self.params.shift_remove(&actual_key);
                self.params.insert(actual_key, decoded);
            }
            // If the target key doesn't exist (e.g., UNICODE__NAME when there's no NAME
            // parameter), the sidecar is still consumed but doesn't produce a new entry.
            // This can happen when the sidecar corresponds to a field stored elsewhere
            // (e.g., NAME comes from the SectionKeys/TOC, not from this parameter block).
        }

        Ok(())
    }

    // Returns the stored key whose lowercase form matches `key`, or `None` if absent.
    fn find_key(&self, key: &str) -> Option<&str> {
        let lower = key.to_ascii_lowercase();
        self.params
            .keys()
            .find(|k| k.to_ascii_lowercase() == lower)
            .map(String::as_str)
    }
}

/// Decodes a UNICODE sidecar value: comma-separated decimal UTF-16 code points.
///
/// Example: `"76,69,68,32,48,54,48,51,29627,29827,32617,46"` decodes to
/// `"LED 0603瓋瑃翉."` (where the large values are CJK code points).
fn decode_unicode_sidecar(key: &str, encoded: &str) -> Result<String> {
    let code_units: Vec<u16> = encoded
        .split(',')
        .filter(|token| !token.trim().is_empty())
        .map(|token| {
            token
                .trim()
                .parse::<u16>()
                .map_err(|_| AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: format!(
                        "invalid UTF-16 code unit in UNICODE sidecar: '{}'",
                        token.trim()
                    ),
                })
        })
        .collect::<Result<Vec<u16>>>()?;
    String::from_utf16(&code_units).map_err(|e| AltiumFormatError::InvalidParamValue {
        key: key.to_owned(),
        detail: format!("UNICODE sidecar UTF-16 decoding failed: {e}"),
    })
}

// Escapes a raw value for Win-1252 encoding (doubleEscape mode).
// Literal Ž (U+017D, Win-1252 byte 0x8E) is doubled so it round-trips as literal 0x8E.
// Pipe | is replaced with Ž (encodes to single 0x8E = escaped pipe).
// Equals = is NOT escaped (parser splits on first = only).
fn escape_for_win1252(s: &str) -> String {
    // Order matters: double literal Ž first, then replace pipes with Ž.
    let s = s.replace('\u{017D}', "\u{017D}\u{017D}");
    s.replace('|', "\u{017D}")
}

// Escapes a raw value for UTF-16LE encoding (broken bar mode).
// Pipe | is replaced with ¦ (broken bar, U+00A6).
// Equals = is NOT escaped.
fn escape_for_utf16le(s: &str) -> String {
    s.replace('|', &String::from(C_SCH_BROKEN_BAR))
}

// Decodes Altium's in-value escape sequences.
// Byte 0x8E (Win-1252 → Ž, U+017D) encodes pipes: single Ž → |, double ŽŽ → literal Ž.
// Broken bar ¦ (U+00A6) is an alternate pipe escape in string/UTF-16LE contexts.
// Equals = is never escaped (parser splits on first = only).
// Verified in `StrUtils.ReplaceSpecialDelimiterChars` and `ProcessMBCSString`.
fn unescape_param_value(s: &str) -> String {
    // Order matters: resolve double Ž first (literal 0x8E), then single Ž (pipe).
    // Windows-1252 decodes byte 0x8E to U+017D (Ž), not U+008E.
    let s = s.replace("\u{017D}\u{017D}", "\x00"); // placeholder for literal Ž
    let s = s.replace('\u{017D}', "|");
    let s = s.replace('\x00', "\u{017D}"); // restore literal Ž
    s.replace(C_SCH_BROKEN_BAR, "|") // broken bar → pipe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_params() {
        let data = b"|RECORD=1|LIBREFERENCE=RES|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let record: i32 = pc.remove_required("RECORD").unwrap();
        assert_eq!(record, 1);
        let lib_ref: String = pc.remove_required("LIBREFERENCE").unwrap();
        assert_eq!(lib_ref, "RES");
        pc.assert_exhausted().unwrap();
    }

    #[test]
    fn case_insensitive_lookup() {
        let data = b"|RECORD=42|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let v: i32 = pc.remove_required("record").unwrap();
        assert_eq!(v, 42);
    }

    #[test]
    fn first_occurrence_wins_duplicates() {
        let data = b"|KEY=first|KEY=second|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let v: String = pc.remove_required("KEY").unwrap();
        assert_eq!(v, "first");
        pc.assert_exhausted().unwrap();
    }

    #[test]
    fn escape_0x8e_single_is_pipe() {
        // 0x8E in Windows-1252 is Ž (U+008E control char in Unicode)
        // After decode, single U+008E → |
        let mut raw = b"|VAL=a".to_vec();
        raw.push(0x8E);
        raw.extend_from_slice(b"b|\0");
        let mut pc = ParameterCollection::from_bytes(&raw).unwrap();
        let v: String = pc.remove_required("VAL").unwrap();
        assert_eq!(v, "a|b");
    }

    #[test]
    fn escape_0x8e_double_is_literal() {
        let mut raw = b"|VAL=a".to_vec();
        raw.push(0x8E);
        raw.push(0x8E);
        raw.extend_from_slice(b"b|\0");
        let mut pc = ParameterCollection::from_bytes(&raw).unwrap();
        let v: String = pc.remove_required("VAL").unwrap();
        // Double 0x8E → literal Ž character (U+017D, Windows-1252 mapping of 0x8E)
        assert_eq!(v, "a\u{017D}b");
    }

    #[test]
    fn escape_broken_bar_is_pipe() {
        // 0xA6 in Windows-1252 is ¦ (broken bar)
        let mut raw = b"|VAL=a".to_vec();
        raw.push(0xA6);
        raw.extend_from_slice(b"b|\0");
        let mut pc = ParameterCollection::from_bytes(&raw).unwrap();
        let v: String = pc.remove_required("VAL").unwrap();
        assert_eq!(v, "a|b");
    }

    #[test]
    fn utf8_key_prefix() {
        // %UTF8%KEY=value where value is UTF-8 encoded
        let data = b"|%UTF8%NAME=caf\xc3\xa9|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let v: String = pc.remove_required("NAME").unwrap();
        assert_eq!(v, "café");
    }

    #[test]
    fn empty_params_from_nul() {
        let data = b"|\0";
        let pc = ParameterCollection::from_bytes(data).unwrap();
        pc.assert_exhausted().unwrap();
    }

    #[test]
    fn missing_param_returns_error() {
        let data = b"|A=1|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let err = pc.remove_required::<i32>("MISSING").unwrap_err();
        assert!(matches!(err, AltiumFormatError::MissingParam(_)));
    }

    #[test]
    fn unknown_params_returns_error() {
        let data = b"|A=1|B=2|\0";
        let pc = ParameterCollection::from_bytes(data).unwrap();
        let err = pc.assert_exhausted().unwrap_err();
        assert!(matches!(err, AltiumFormatError::UnknownParams { .. }));
    }

    #[test]
    fn remove_coord_with_frac() {
        let data = b"|LOCATION.X=100|LOCATION.X_FRAC=50000|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let c = pc.remove_coord("LOCATION.X", "LOCATION.X_FRAC").unwrap();
        // 100 * 100_000 + 50_000 = 10_050_000
        assert_eq!(c.to_internal(), 10_050_000);
        pc.assert_exhausted().unwrap();
    }

    #[test]
    fn remove_coord_without_frac() {
        let data = b"|LOCATION.X=100|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let c = pc.remove_coord("LOCATION.X", "LOCATION.X_FRAC").unwrap();
        assert_eq!(c.to_internal(), 10_000_000);
        pc.assert_exhausted().unwrap();
    }

    #[test]
    fn remove_optional_present() {
        let data = b"|FOO=42|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let v: Option<i32> = pc.remove_optional("FOO").unwrap();
        assert_eq!(v, Some(42));
    }

    #[test]
    fn remove_optional_absent() {
        let data = b"|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let v: Option<i32> = pc.remove_optional("FOO").unwrap();
        assert_eq!(v, None);
    }

    #[test]
    fn remove_with_default_present() {
        let data = b"|X=5|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let v: i32 = pc.remove_with_default("X", 99).unwrap();
        assert_eq!(v, 5);
    }

    #[test]
    fn remove_with_default_absent() {
        let data = b"|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let v: i32 = pc.remove_with_default("X", 99).unwrap();
        assert_eq!(v, 99);
    }

    #[test]
    fn bool_param_values() {
        let data = b"|A=T|B=F|C=TRUE|D=FALSE|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        assert!(pc.remove_required::<bool>("A").unwrap());
        assert!(!pc.remove_required::<bool>("B").unwrap());
        assert!(pc.remove_required::<bool>("C").unwrap());
        assert!(!pc.remove_required::<bool>("D").unwrap());
    }

    #[test]
    fn from_utf16le_bytes() {
        // "|KEY=value|\0" encoded as UTF-16LE
        let s = "|KEY=value|\0";
        let utf16: Vec<u8> = s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        let mut pc = ParameterCollection::from_utf16le_bytes(&utf16).unwrap();
        let v: String = pc.remove_required("KEY").unwrap();
        assert_eq!(v, "value");
    }

    #[test]
    fn remove_indexed_coords() {
        let data = b"|COUNT=2|X1=1|Y1=2|X2=3|Y2=4|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let points = pc.remove_indexed_coords("COUNT", "X", "Y").unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].x.to_internal(), 100_000); // 1 * 100_000
        assert_eq!(points[0].y.to_internal(), 200_000);
        assert_eq!(points[1].x.to_internal(), 300_000);
        assert_eq!(points[1].y.to_internal(), 400_000);
        pc.assert_exhausted().unwrap();
    }

    #[test]
    fn remove_list() {
        let data = b"|ITEMS=1,2,3|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let items: Vec<i32> = pc.remove_list("ITEMS").unwrap();
        assert_eq!(items, vec![1, 2, 3]);
    }

    #[test]
    fn remove_list_or_empty_absent() {
        let data = b"|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let items: Vec<i32> = pc.remove_list_or_empty("ITEMS").unwrap();
        assert!(items.is_empty());
    }

    // ── Serialization tests ──────────────────────────────────────────

    #[test]
    fn to_bytes_simple_roundtrip() {
        let mut pc = ParameterCollection::new();
        pc.insert("RECORD", "1".to_owned());
        pc.insert("KEY", "value".to_owned());
        let bytes = pc.to_bytes();
        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let record: i32 = pc2.remove_required("RECORD").unwrap();
        assert_eq!(record, 1);
        let key: String = pc2.remove_required("KEY").unwrap();
        assert_eq!(key, "value");
        pc2.assert_exhausted().unwrap();
    }

    #[test]
    fn to_bytes_pipe_in_value_roundtrip() {
        // Value contains pipe → triggers dual-write (¦ > '~').
        let mut pc = ParameterCollection::new();
        pc.insert("VAL", "a|b=c".to_owned());
        let bytes = pc.to_bytes();
        // Verify dual-write: UTF-8 version uses ¦ (broken bar), Win-1252 uses 0x8E (Ž)
        assert!(
            bytes.windows(6).any(|w| w == b"%UTF8%"),
            "should contain %UTF8% prefix"
        );
        assert!(bytes.contains(&0x8E), "should contain 0x8E escape byte");
        // Roundtrip: parsing gives back original value
        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let v: String = pc2.remove_required("VAL").unwrap();
        assert_eq!(v, "a|b=c");
    }

    #[test]
    fn to_bytes_non_ascii_dual_write() {
        // Non-ASCII µ triggers %UTF8% dual-write.
        let mut pc = ParameterCollection::new();
        pc.insert("Text", "0.1\u{00B5}F".to_owned()); // µF
        let bytes = pc.to_bytes();
        // UTF-8 version: µ as C2 B5
        assert!(
            bytes.windows(6).any(|w| w == b"%UTF8%"),
            "should contain %UTF8% prefix"
        );
        assert!(
            bytes.windows(2).any(|w| w == [0xC2, 0xB5]),
            "should contain UTF-8 µ"
        );
        // Win-1252 version: µ as single byte B5
        assert!(bytes.contains(&0xB5), "should contain Win-1252 µ byte");
        // Roundtrip
        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let v: String = pc2.remove_required("Text").unwrap();
        assert_eq!(v, "0.1\u{00B5}F");
    }

    #[test]
    fn to_bytes_ascii_no_dual_write() {
        // Pure ASCII value should NOT trigger dual-write.
        let mut pc = ParameterCollection::new();
        pc.insert("KEY", "hello".to_owned());
        let bytes = pc.to_bytes();
        assert!(
            !bytes.windows(6).any(|w| w == b"%UTF8%"),
            "pure ASCII should not have %UTF8%"
        );
        assert_eq!(&bytes, b"|KEY=hello\0");
    }

    #[test]
    fn to_bytes_equals_not_escaped() {
        // Equals in value is NOT escaped — parser splits on first = only.
        let mut pc = ParameterCollection::new();
        pc.insert("VAL", "x=y=z".to_owned());
        let bytes = pc.to_bytes();
        // The raw bytes should contain literal = signs in the value
        assert_eq!(&bytes, b"|VAL=x=y=z\0");
        // Roundtrip
        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let v: String = pc2.remove_required("VAL").unwrap();
        assert_eq!(v, "x=y=z");
    }

    #[test]
    fn to_bytes_trailing_0x8e_boundary_guard() {
        // Value ending with | → escaped to trailing Ž (0x8E).
        // Boundary guard appends 0x7C after the 0x8E.
        let mut pc = ParameterCollection::new();
        pc.insert("VAL", "a|".to_owned());
        let bytes = pc.to_bytes();
        // Find the Win-1252 portion: should end with 0x8E 0x7C before NUL
        // (boundary guard pipe after trailing escape byte)
        let nul_pos = bytes.iter().rposition(|&b| b == 0).unwrap();
        assert_eq!(bytes[nul_pos - 1], b'|', "boundary guard pipe before NUL");
        assert_eq!(bytes[nul_pos - 2], 0x8E, "0x8E escape before guard pipe");
        // Roundtrip
        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let v: String = pc2.remove_required("VAL").unwrap();
        assert_eq!(v, "a|");
    }

    #[test]
    fn to_bytes_empty_collection() {
        let pc = ParameterCollection::new();
        let bytes = pc.to_bytes();
        assert_eq!(bytes, b"\0");
    }

    #[test]
    fn insert_coord_with_frac() {
        let mut pc = ParameterCollection::new();
        pc.insert_coord("LOC.X", "LOC.X_FRAC", Coord::from_internal(10_050_000));
        let bytes = pc.to_bytes();
        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let c = pc2.remove_coord("LOC.X", "LOC.X_FRAC").unwrap();
        assert_eq!(c.to_internal(), 10_050_000);
        pc2.assert_exhausted().unwrap();
    }

    #[test]
    fn insert_coord_without_frac() {
        let mut pc = ParameterCollection::new();
        pc.insert_coord("LOC.X", "LOC.X_FRAC", Coord::from_internal(10_000_000));
        let bytes = pc.to_bytes();
        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let c = pc2.remove_coord("LOC.X", "LOC.X_FRAC").unwrap();
        assert_eq!(c.to_internal(), 10_000_000);
        pc2.assert_exhausted().unwrap();
        // Verify no FRAC key was written
        let raw = std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap();
        assert!(!raw.contains("FRAC"), "FRAC should not appear when frac=0");
    }

    #[test]
    fn insert_coord_point_roundtrip() {
        let mut pc = ParameterCollection::new();
        let point = CoordPoint::new(
            Coord::from_internal(10_050_000),
            Coord::from_internal(20_075_000),
        );
        pc.insert_coord_point(
            "Location.X",
            "Location.X_FRAC",
            "Location.Y",
            "Location.Y_FRAC",
            point,
        );
        let bytes = pc.to_bytes();
        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let x = pc2.remove_coord("Location.X", "Location.X_FRAC").unwrap();
        let y = pc2.remove_coord("Location.Y", "Location.Y_FRAC").unwrap();
        assert_eq!(x.to_internal(), 10_050_000);
        assert_eq!(y.to_internal(), 20_075_000);
        pc2.assert_exhausted().unwrap();
    }

    #[test]
    fn insert_indexed_coords_roundtrip() {
        let mut pc = ParameterCollection::new();
        let points = vec![
            CoordPoint::new(Coord::from_internal(100_000), Coord::from_internal(200_000)),
            CoordPoint::new(Coord::from_internal(300_000), Coord::from_internal(400_000)),
        ];
        pc.insert_indexed_coords("LocationCount", "X", "Y", &points);
        let bytes = pc.to_bytes();
        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let parsed_points = pc2
            .remove_indexed_coords("LocationCount", "X", "Y")
            .unwrap();
        assert_eq!(parsed_points.len(), 2);
        assert_eq!(parsed_points[0].x.to_internal(), 100_000);
        assert_eq!(parsed_points[0].y.to_internal(), 200_000);
        assert_eq!(parsed_points[1].x.to_internal(), 300_000);
        assert_eq!(parsed_points[1].y.to_internal(), 400_000);
        pc2.assert_exhausted().unwrap();
    }

    #[test]
    fn indexed_coords_overflow_roundtrip() {
        // 53 vertices: first 50 go under LocationCount+X/Y, last 3 under EXTRALOCATIONCOUNT+EX/EY.
        let points: Vec<CoordPoint> = (1..=53)
            .map(|i| {
                CoordPoint::new(
                    Coord::from_internal(i * 100_000),
                    Coord::from_internal(i * 200_000),
                )
            })
            .collect();

        let mut pc = ParameterCollection::new();
        pc.insert_indexed_coords("LocationCount", "X", "Y", &points);
        let bytes = pc.to_bytes();
        let raw = std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap();

        // Verify serialization split
        assert!(raw.contains("LocationCount=50"), "base count should be 50");
        assert!(
            raw.contains("EXTRALOCATIONCOUNT=3"),
            "extra count should be 3"
        );
        assert!(raw.contains("EX51="), "should have EX51");
        assert!(raw.contains("EY53="), "should have EY53");
        assert!(
            !raw.contains("|X51="),
            "should NOT have X51 (overflow uses EX prefix)"
        );

        // Roundtrip parse
        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let parsed = pc2
            .remove_indexed_coords("LocationCount", "X", "Y")
            .unwrap();
        assert_eq!(parsed.len(), 53);
        for (i, pt) in parsed.iter().enumerate() {
            let n = (i + 1) as i32;
            assert_eq!(pt.x.to_internal(), n * 100_000, "x mismatch at vertex {n}");
            assert_eq!(pt.y.to_internal(), n * 200_000, "y mismatch at vertex {n}");
        }
        pc2.assert_exhausted().unwrap();
    }

    #[test]
    fn indexed_coords_exactly_50_no_overflow() {
        // Exactly 50 vertices: no EXTRALOCATIONCOUNT should be emitted.
        let points: Vec<CoordPoint> = (1..=50)
            .map(|i| {
                CoordPoint::new(
                    Coord::from_internal(i * 100_000),
                    Coord::from_internal(i * 200_000),
                )
            })
            .collect();

        let mut pc = ParameterCollection::new();
        pc.insert_indexed_coords("LocationCount", "X", "Y", &points);
        let bytes = pc.to_bytes();
        let raw = std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap();

        assert!(raw.contains("LocationCount=50"));
        assert!(
            !raw.contains("EXTRALOCATIONCOUNT"),
            "should not have overflow for exactly 50"
        );

        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let parsed = pc2
            .remove_indexed_coords("LocationCount", "X", "Y")
            .unwrap();
        assert_eq!(parsed.len(), 50);
        pc2.assert_exhausted().unwrap();
    }

    #[test]
    fn remove_indexed_coords_with_extra_location_count() {
        // Simulate what Altium writes for 52 vertices: manual parameter construction.
        let mut params = String::from("|LocationCount=50");
        for i in 1..=50 {
            params.push_str(&format!("|X{i}={i}|Y{i}={}", i * 2));
        }
        params.push_str("|EXTRALOCATIONCOUNT=2");
        params.push_str("|EX51=51|EY51=102|EX52=52|EY52=104");
        params.push_str("|\0");

        let mut pc = ParameterCollection::from_bytes(params.as_bytes()).unwrap();
        let points = pc.remove_indexed_coords("LocationCount", "X", "Y").unwrap();
        assert_eq!(points.len(), 52);
        // Check vertex 51 (first overflow)
        assert_eq!(points[50].x.to_internal(), 51 * 100_000);
        assert_eq!(points[50].y.to_internal(), 102 * 100_000);
        // Check vertex 52 (second overflow)
        assert_eq!(points[51].x.to_internal(), 52 * 100_000);
        assert_eq!(points[51].y.to_internal(), 104 * 100_000);
        pc.assert_exhausted().unwrap();
    }

    #[test]
    fn insert_preserves_order() {
        let mut pc = ParameterCollection::new();
        pc.insert("RECORD", "1".to_owned());
        pc.insert("NAME", "test".to_owned());
        pc.insert("VALUE", "42".to_owned());
        let bytes = pc.to_bytes();
        let s = std::str::from_utf8(&bytes[..bytes.len() - 1])
            .expect("to_bytes should produce valid UTF-8");
        // Order must be: |RECORD=1|NAME=test|VALUE=42 (no trailing pipe)
        assert_eq!(s, "|RECORD=1|NAME=test|VALUE=42", "got: {s}");
    }

    #[test]
    fn insert_negative_coord() {
        let mut pc = ParameterCollection::new();
        // Negative coord: -5 DXP units + 50000 frac = -5*100000 + 50000 = -450000
        pc.insert_coord("LOC.X", "LOC.X_FRAC", Coord::from_internal(-450_000));
        let bytes = pc.to_bytes();
        let mut pc2 = ParameterCollection::from_bytes(&bytes).unwrap();
        let c = pc2.remove_coord("LOC.X", "LOC.X_FRAC").unwrap();
        assert_eq!(c.to_internal(), -450_000);
    }

    #[test]
    fn remaining_keys_and_count() {
        let data = b"|A=1|B=2|C=3|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        assert_eq!(pc.remaining_count(), 3);
        let _: i32 = pc.remove_required("A").unwrap();
        assert_eq!(pc.remaining_count(), 2);
        let keys = pc.remaining_keys();
        assert!(keys.contains(&"B"));
        assert!(keys.contains(&"C"));
    }
}
