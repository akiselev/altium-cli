//! Layer 4 parameter collection for text-format Altium blocks.
//! Pipe-delimited key=value pairs decoded from Windows-1252 bytes.
//! Keys stored in original case; lookups are case-insensitive.
//! Accessors are destructive (remove-on-read): `assert_exhausted` then
//! confirms every key was consumed, enforcing the fail-fast invariant.
//! Insertion order is preserved (IndexMap) for deterministic serialization.
use indexmap::IndexMap;
use altium_format_types::{Coord, CoordPoint};

use crate::param_value::FromParamValue;
use crate::{AltiumFormatError, Result};

pub(crate) struct ParameterCollection {
    // Keys stored in original case for round-trip fidelity; lookups are case-insensitive.
    // IndexMap preserves insertion order for deterministic serialization.
    params: IndexMap<String, String>,
}

impl ParameterCollection {
    // Creates an empty collection; use from_bytes or from_utf16le_bytes to populate.
    pub(crate) fn new() -> Self {
        Self { params: IndexMap::new() }
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
            let value_str = if key_str.starts_with("%UTF8%") {
                let stripped_key = key_str[6..].to_owned();
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
                let (decoded, _) =
                    encoding_rs::WINDOWS_1252.decode_without_bom_handling(raw_value);
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
        let (decoded, _) = encoding_rs::UTF_16LE.decode_without_bom_handling(data);
        Self::from_str_params(&decoded)
    }

    // Treats all values as already-decoded strings; %UTF8% key prefix handling
    // does not apply here. Only from_bytes (raw-byte path) strips %UTF8% and
    // switches to UTF-8 decoding for the value bytes.
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
                let value = self.params.shift_remove(&actual_key).unwrap();
                T::from_param_value(&actual_key, &value)
            }
            None => Err(AltiumFormatError::MissingParam(key.to_owned())),
        }
    }

    // Removes key (case-insensitive) and parses it if present; returns Ok(None) if absent.
    pub(crate) fn remove_optional<T: FromParamValue>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>> {
        let found = self.find_key(key).map(|k| k.to_owned());
        match found {
            Some(actual_key) => {
                let value = self.params.shift_remove(&actual_key).unwrap();
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
        let integer: i32 = self.remove_required(key)?;
        let frac: i32 = self.remove_with_default(frac_key, 0i32)?;
        Ok(Coord::from_dxp_frac(integer, frac))
    }

    // Reads count from `count_key`, then removes `{x_prefix}N`/`{y_prefix}N` pairs as Coords.
    pub(crate) fn remove_indexed_coords(
        &mut self,
        count_key: &str,
        x_prefix: &str,
        y_prefix: &str,
    ) -> Result<Vec<CoordPoint>> {
        let count: usize = self.remove_required(count_key)?;
        let mut points = Vec::with_capacity(count);
        for i in 0..count {
            let x_key = format!("{x_prefix}{i}");
            let y_key = format!("{y_prefix}{i}");
            let x_frac_key = format!("{x_prefix}{i}_FRAC");
            let y_frac_key = format!("{y_prefix}{i}_FRAC");
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
    pub(crate) fn remove_list_or_empty<T: FromParamValue>(
        &mut self,
        key: &str,
    ) -> Result<Vec<T>> {
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

    // Returns Err(UnknownParams) if any keys remain unconsumed.
    // Call at the dispatch boundary after all known fields are removed.
    pub(crate) fn assert_exhausted(&self) -> Result<()> {
        if self.params.is_empty() {
            return Ok(());
        }
        let keys: Vec<String> = self.params.keys().cloned().collect();
        Err(AltiumFormatError::UnknownParams { keys })
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

// Decodes Altium's in-value escape sequences.
// Altium encodes literal pipe and equals inside values because | and = are delimiters.
// Additionally, byte 0x8E (142) encodes a literal pipe within values (single 0x8E → |,
// double 0x8E 0x8E → literal 0x8E character). Byte 0xA6 (broken bar ¦) is an alternate
// pipe escape in ASCII format (¦ → |). Verified in `StrUtils.ReplaceSpecialDelimiterChars`
// and `ProcessMBCSString` in decompiled .NET source.
fn unescape_param_value(s: &str) -> String {
    // Order matters: resolve double-0x8E first (literal 0x8E), then single 0x8E (pipe).
    // Windows-1252 decodes byte 0x8E to U+017D (Ž), not U+008E.
    let s = s.replace("\u{017D}\u{017D}", "\x00");  // placeholder for literal Ž
    let s = s.replace('\u{017D}', "|");
    let s = s.replace('\x00', "\u{017D}");           // restore literal Ž (0x8E in Windows-1252)
    let s = s.replace('\u{00a6}', "|");               // broken bar → pipe
    s.replace("[]", "|").replace("{}", "=")
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
    fn escape_brackets_and_braces() {
        let data = b"|VAL=a[]b{}c|\0";
        let mut pc = ParameterCollection::from_bytes(data).unwrap();
        let v: String = pc.remove_required("VAL").unwrap();
        assert_eq!(v, "a|b=c");
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
        let data = b"|COUNT=2|X0=1|Y0=2|X1=3|Y1=4|\0";
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
