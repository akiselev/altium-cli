//! Serialization traits and primitive implementations for the v2 API.
//!
//! The core traits defined here:
//!
//! - [`ParamCodec`] — read/write a typed value from/to a [`ParameterCollection`] by key.
//! - [`AltiumEnum`] — integer-backed enum conversion, with a macro-generated [`ParamCodec`] impl.
//! - [`RecordType`] — marker trait for macro-generated record types.
//! - [`WrapperFamily`] — ties a record type to its view type for query type parameters.
//!
//! Primitive `ParamCodec` implementations cover `String`, `i32`, `i16`, `u8`, `u32`,
//! `bool`, `f64`, and `Option<T>`.
//!
//! Coordinate implementations (`SchCoord`, `PcbCoord`) depend on types from
//! [`crate::v2::coord`] (Track 1A). They are included here with the expectation
//! that Track 1A will provide the concrete types with `from_dxp_parts`,
//! `to_dxp_parts`, `from_raw`, and `to_raw` methods.

use crate::v2::backing_store::RecordOrigin;
use crate::v2::parameters::ParameterCollection;

// ---------------------------------------------------------------------------
// ParamCodec
// ---------------------------------------------------------------------------

/// Trait for types that can read/write themselves from/to parameter collections.
///
/// The `key` argument is the base param key name. Types that need additional
/// related keys (e.g., `SchCoord` needs `{key}_FRAC`) derive them internally.
pub trait ParamCodec: Sized {
    /// Read a value from `params` under the given `key`.
    ///
    /// Returns `None` if the key is absent from the collection.
    fn read(params: &ParameterCollection, key: &str) -> Option<Self>;

    /// Write this value into `params` under the given `key`.
    fn write(&self, params: &mut ParameterCollection, key: &str);
}

// ---------------------------------------------------------------------------
// AltiumEnum
// ---------------------------------------------------------------------------

/// Trait for Altium enums that map to/from integer values in parameter files.
///
/// Types implementing this trait should use the [`impl_altium_enum_codec!`]
/// macro to generate the corresponding [`ParamCodec`] implementation.
///
/// A blanket `ParamCodec` impl is not possible in stable Rust due to coherence
/// rules conflicting with the primitive `ParamCodec` impls. The macro provides
/// the same ergonomics.
pub trait AltiumEnum: Sized {
    /// Convert from an integer value. Unknown values should map to a default
    /// or `Unknown` variant.
    fn from_int(value: i32) -> Self;

    /// Convert to the integer representation.
    fn to_int(&self) -> i32;
}

/// Generates a [`ParamCodec`] implementation for a type that implements
/// [`AltiumEnum`]. Reads/writes through the integer representation.
///
/// # Example
///
/// ```ignore
/// impl AltiumEnum for PinElectricalType {
///     fn from_int(value: i32) -> Self { /* ... */ }
///     fn to_int(&self) -> i32 { /* ... */ }
/// }
/// impl_altium_enum_codec!(PinElectricalType);
/// ```
#[macro_export]
macro_rules! impl_altium_enum_codec {
    ($ty:ty) => {
        impl $crate::v2::traits::ParamCodec for $ty {
            fn read(
                params: &$crate::v2::parameters::ParameterCollection,
                key: &str,
            ) -> Option<Self> {
                use $crate::v2::traits::AltiumEnum;
                params.get(key).map(|v| Self::from_int(v.as_int_or(0)))
            }

            fn write(
                &self,
                params: &mut $crate::v2::parameters::ParameterCollection,
                key: &str,
            ) {
                use $crate::v2::traits::AltiumEnum;
                params.add_int(key, self.to_int());
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Primitive ParamCodec implementations
// ---------------------------------------------------------------------------

// -- String --

impl ParamCodec for String {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        params.get(key).map(|v| v.as_str().to_string())
    }

    fn write(&self, params: &mut ParameterCollection, key: &str) {
        params.add(key, self);
    }
}

// -- i32 --

impl ParamCodec for i32 {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        params.get(key).map(|v| v.as_int_or(0))
    }

    fn write(&self, params: &mut ParameterCollection, key: &str) {
        params.add_int(key, *self);
    }
}

// -- i16 (cast through i32) --

impl ParamCodec for i16 {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        params.get(key).map(|v| v.as_int_or(0) as i16)
    }

    fn write(&self, params: &mut ParameterCollection, key: &str) {
        params.add_int(key, *self as i32);
    }
}

// -- u8 (cast through i32) --

impl ParamCodec for u8 {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        params.get(key).map(|v| v.as_int_or(0) as u8)
    }

    fn write(&self, params: &mut ParameterCollection, key: &str) {
        params.add_int(key, *self as i32);
    }
}

// -- u32 (cast through i32, for color values etc.) --

impl ParamCodec for u32 {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        params.get(key).map(|v| v.as_int_or(0) as u32)
    }

    fn write(&self, params: &mut ParameterCollection, key: &str) {
        params.add_int(key, *self as i32);
    }
}

// -- bool (T/F string format) --

impl ParamCodec for bool {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        params.get(key).map(|v| v.as_bool_or(false))
    }

    fn write(&self, params: &mut ParameterCollection, key: &str) {
        // Write explicitly as "T" or "F" string so that false values are
        // preserved in the parameter collection. Note: ParameterCollection::add_bool
        // only writes when true, which would lose explicit false values.
        params.add(key, if *self { "T" } else { "F" });
    }
}

// -- f64 --

impl ParamCodec for f64 {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        params.get(key).map(|v| v.as_double_or(0.0))
    }

    fn write(&self, params: &mut ParameterCollection, key: &str) {
        // Use enough decimal places to preserve precision.
        // Altium typically uses up to 6 decimal places.
        params.add_double(key, *self, 6);
    }
}

// -- Option<T: ParamCodec> --

impl<T: ParamCodec> ParamCodec for Option<T> {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        // Always returns Some — the outer Option represents "did the codec run",
        // the inner Option represents "was the key present".
        // If the key is missing, returns Some(None).
        // If the key is present, delegates to T::read and wraps in Some.
        Some(T::read(params, key))
    }

    fn write(&self, params: &mut ParameterCollection, key: &str) {
        if let Some(inner) = self {
            inner.write(params, key);
        }
        // If None, don't write anything — the key stays absent.
    }
}

// ---------------------------------------------------------------------------
// Coordinate ParamCodec implementations
// ---------------------------------------------------------------------------

use crate::v2::coord::{AltiumCoord, PcbCoord, SchCoord};

impl ParamCodec for SchCoord {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        let int_val = params.get(key)?.as_int_or(0);
        let frac_key = format!("{}_FRAC", key);
        let frac_val = params
            .get(&frac_key)
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);
        Some(SchCoord::from_dxp_parts(int_val, frac_val))
    }

    fn write(&self, params: &mut ParameterCollection, key: &str) {
        let (int_val, frac_val) = self.to_dxp_parts();
        params.add_int(key, int_val);
        if frac_val != 0 {
            params.add_int(&format!("{}_FRAC", key), frac_val);
        }
    }
}

impl ParamCodec for PcbCoord {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        params
            .get(key)
            .map(|v| PcbCoord::from_raw(v.as_int_or(0)))
    }

    fn write(&self, params: &mut ParameterCollection, key: &str) {
        params.add_int(key, self.to_raw());
    }
}

// ---------------------------------------------------------------------------
// RecordType trait
// ---------------------------------------------------------------------------

/// Marker trait for macro-generated record types.
///
/// Every record struct generated by the `#[altium_record]` attribute macro
/// implements this trait. It provides the record's integer ID and access to
/// the underlying [`RecordOrigin`] backing store.
pub trait RecordType {
    /// The numeric record identifier (RECORD param value for schematic types,
    /// type byte for PCB binary types).
    const RECORD_ID: u8;

    /// Whether this record type uses a binary origin (`true` for PCB records,
    /// `false` for schematic parameter-based records).
    const IS_BINARY: bool;

    /// Shared reference to the backing store origin.
    fn origin(&self) -> &RecordOrigin;

    /// Mutable reference to the backing store origin.
    fn origin_mut(&mut self) -> &mut RecordOrigin;
}

// ---------------------------------------------------------------------------
// FromOrigin trait
// ---------------------------------------------------------------------------

/// Trait for record types that can be constructed from / decomposed into
/// a [`RecordOrigin`].
///
/// The proc macro already generates `from_origin()` on each record struct.
/// This trait formalises the contract so generic code (handles, store) can
/// work with any record type.
pub trait FromOrigin: Sized {
    /// Create a record from a backing-store origin.
    fn from_origin(origin: RecordOrigin) -> Self;
    /// Consume the record and return its backing-store origin.
    fn into_origin(self) -> RecordOrigin;
}

// ---------------------------------------------------------------------------
// HandleFamily trait
// ---------------------------------------------------------------------------

/// Associates a record type with its handle type, used as a type parameter
/// for the query and handle APIs.
///
/// Users pass `HandleFamily` implementors as type parameters to
/// `comp.children::<SchPin>()` and similar methods. The trait connects
/// the family marker type to the concrete record and handle types.
pub trait HandleFamily {
    /// The underlying record type.
    type Record: RecordType + FromOrigin;
    /// The handle type (Clone, holds DocRef + RecordId).
    type Handle: Clone;

    /// Construct a handle from a store reference and record id.
    fn make_handle(store: crate::v2::store::DocRef, id: crate::v2::ids::RecordId) -> Self::Handle;

    /// Convenience: returns the record ID from the associated record type.
    fn record_id() -> u8 {
        Self::Record::RECORD_ID
    }

    /// Convenience: returns whether this family expects a binary origin.
    fn is_binary() -> bool {
        Self::Record::IS_BINARY
    }
}

// ---------------------------------------------------------------------------
// DocumentQuery trait (new: takes &self, no closures)
// ---------------------------------------------------------------------------

/// Trait for document-level querying, parameterized by the target handle
/// family type.
///
/// Documents implement this for each type they support querying. Queries
/// take `&self` (shared access via `Rc<RefCell<>>`), not `&mut self`.
pub trait DocumentQuery<T: HandleFamily> {
    /// Query for a single match. Returns `NoMatch` or `AmbiguousMatch` on failure.
    fn query(&self, q: &str) -> crate::error::Result<T::Handle>;
    /// Query for all matches.
    fn query_all(&self, q: &str) -> crate::error::Result<Vec<T::Handle>>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::parameters::ParameterCollection;

    #[test]
    fn param_codec_string() {
        let mut params = ParameterCollection::new();

        // Write
        let value = String::from("Resistor");
        value.write(&mut params, "LIBREFERENCE");

        // Read back
        let read_back = String::read(&params, "LIBREFERENCE");
        assert_eq!(read_back, Some("Resistor".to_string()));

        // Missing key returns None
        let missing = String::read(&params, "NONEXISTENT");
        assert_eq!(missing, None);
    }

    #[test]
    fn param_codec_int() {
        let mut params = ParameterCollection::new();

        // Write
        let value: i32 = 42;
        value.write(&mut params, "RECORD");

        // Read back
        let read_back = i32::read(&params, "RECORD");
        assert_eq!(read_back, Some(42));

        // Missing key returns None
        let missing = i32::read(&params, "NONEXISTENT");
        assert_eq!(missing, None);
    }

    #[test]
    fn param_codec_int_zero() {
        let mut params = ParameterCollection::new();

        // Write zero — add_int skips zero values, so reading back gives None.
        let value: i32 = 0;
        value.write(&mut params, "OFFSET");

        let read_back = i32::read(&params, "OFFSET");
        assert_eq!(read_back, None);
    }

    #[test]
    fn param_codec_i16() {
        let mut params = ParameterCollection::new();

        let value: i16 = -123;
        value.write(&mut params, "ROTATION");

        let read_back = i16::read(&params, "ROTATION");
        assert_eq!(read_back, Some(-123));
    }

    #[test]
    fn param_codec_u8() {
        let mut params = ParameterCollection::new();

        let value: u8 = 255;
        value.write(&mut params, "LAYER");

        let read_back = u8::read(&params, "LAYER");
        // 255 written as i32 (255), read as i32 (255), cast to u8 (255)
        assert_eq!(read_back, Some(255));
    }

    #[test]
    fn param_codec_u32() {
        let mut params = ParameterCollection::new();

        let value: u32 = 0x00FF8040;
        value.write(&mut params, "COLOR");

        let read_back = u32::read(&params, "COLOR");
        assert_eq!(read_back, Some(0x00FF8040));
    }

    #[test]
    fn param_codec_bool() {
        let mut params = ParameterCollection::new();

        // Write true
        true.write(&mut params, "VISIBLE");
        let read_back = bool::read(&params, "VISIBLE");
        assert_eq!(read_back, Some(true));

        // Write false
        false.write(&mut params, "LOCKED");
        let read_back = bool::read(&params, "LOCKED");
        assert_eq!(read_back, Some(false));

        // Missing key returns None
        let missing = bool::read(&params, "NONEXISTENT");
        assert_eq!(missing, None);
    }

    #[test]
    fn param_codec_f64() {
        let mut params = ParameterCollection::new();

        let value: f64 = 3.141593;
        value.write(&mut params, "ANGLE");

        let read_back = f64::read(&params, "ANGLE");
        assert!(read_back.is_some());
        let diff = (read_back.unwrap() - 3.141593).abs();
        assert!(diff < 1e-5, "f64 roundtrip error: {diff}");
    }

    #[test]
    fn param_codec_f64_zero() {
        let mut params = ParameterCollection::new();

        // add_double skips zero values
        let value: f64 = 0.0;
        value.write(&mut params, "SCALE");

        let read_back = f64::read(&params, "SCALE");
        assert_eq!(read_back, None);
    }

    #[test]
    fn param_codec_option() {
        let mut params = ParameterCollection::new();

        // Write Some value
        let value: Option<String> = Some("hello".to_string());
        value.write(&mut params, "DESC");

        // Read back — returns Some(Some("hello"))
        let read_back = Option::<String>::read(&params, "DESC");
        assert_eq!(read_back, Some(Some("hello".to_string())));

        // Missing key — returns Some(None)
        let missing = Option::<String>::read(&params, "NONEXISTENT");
        assert_eq!(missing, Some(None));

        // Write None — key should not be added
        let mut params2 = ParameterCollection::new();
        let none_val: Option<String> = None;
        none_val.write(&mut params2, "DESC");
        assert!(!params2.contains("DESC"));
    }

    #[test]
    fn param_codec_option_int() {
        let mut params = ParameterCollection::new();
        params.add("COUNT", "5");

        let read_back = Option::<i32>::read(&params, "COUNT");
        assert_eq!(read_back, Some(Some(5)));

        let missing = Option::<i32>::read(&params, "MISSING");
        assert_eq!(missing, Some(None));
    }

    // -- AltiumEnum tests via the macro --

    #[derive(Debug, PartialEq, Eq)]
    enum TestOrientation {
        Up,
        Right,
        Down,
        Left,
        Unknown(i32),
    }

    impl AltiumEnum for TestOrientation {
        fn from_int(value: i32) -> Self {
            match value {
                0 => TestOrientation::Up,
                1 => TestOrientation::Right,
                2 => TestOrientation::Down,
                3 => TestOrientation::Left,
                other => TestOrientation::Unknown(other),
            }
        }

        fn to_int(&self) -> i32 {
            match self {
                TestOrientation::Up => 0,
                TestOrientation::Right => 1,
                TestOrientation::Down => 2,
                TestOrientation::Left => 3,
                TestOrientation::Unknown(v) => *v,
            }
        }
    }

    impl_altium_enum_codec!(TestOrientation);

    #[test]
    fn param_codec_altium_enum() {
        let mut params = ParameterCollection::new();

        let value = TestOrientation::Right;
        value.write(&mut params, "ORIENTATION");

        let read_back = TestOrientation::read(&params, "ORIENTATION");
        assert_eq!(read_back, Some(TestOrientation::Right));

        // Unknown value
        params.add("ORIENTATION", "99");
        let read_back = TestOrientation::read(&params, "ORIENTATION");
        assert_eq!(read_back, Some(TestOrientation::Unknown(99)));
    }

    #[test]
    fn param_codec_altium_enum_missing() {
        let params = ParameterCollection::new();
        let read_back = TestOrientation::read(&params, "ORIENTATION");
        assert_eq!(read_back, None);
    }
}
