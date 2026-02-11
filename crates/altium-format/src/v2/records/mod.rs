//! Phase 3

#[cfg(test)]
mod tests {
    // -----------------------------------------------------------------------
    // altium_enum tests
    // -----------------------------------------------------------------------

    #[altium_format_derive::altium_enum]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum TestOrientation {
        Up = 0,
        Right = 1,
        Down = 2,
        Left = 3,
    }

    #[test]
    fn enum_from_int() {
        use crate::v2::traits::AltiumEnum;
        assert_eq!(TestOrientation::from_int(0), TestOrientation::Up);
        assert_eq!(TestOrientation::from_int(1), TestOrientation::Right);
        assert_eq!(TestOrientation::from_int(2), TestOrientation::Down);
        assert_eq!(TestOrientation::from_int(3), TestOrientation::Left);
    }

    #[test]
    fn enum_to_int() {
        use crate::v2::traits::AltiumEnum;
        assert_eq!(TestOrientation::Up.to_int(), 0);
        assert_eq!(TestOrientation::Right.to_int(), 1);
    }

    #[test]
    fn enum_fallback() {
        use crate::v2::traits::AltiumEnum;
        // Unknown value falls back to first variant
        assert_eq!(TestOrientation::from_int(99), TestOrientation::Up);
    }

    #[test]
    fn enum_param_codec() {
        use crate::v2::parameters::ParameterCollection;
        use crate::v2::traits::ParamCodec;

        let mut params = ParameterCollection::new();
        TestOrientation::Right.write(&mut params, "ORIENTATION");

        let read_back = TestOrientation::read(&params, "ORIENTATION");
        assert_eq!(read_back, Some(TestOrientation::Right));
    }

    // Test with custom fallback
    #[altium_format_derive::altium_enum(fallback = "Unknown")]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum TestWithFallback {
        Normal = 0,
        Special = 1,
        Unknown = 255,
    }

    #[test]
    fn enum_custom_fallback() {
        use crate::v2::traits::AltiumEnum;
        assert_eq!(TestWithFallback::from_int(99), TestWithFallback::Unknown);
    }

    // Test with explicit altium(value) attribute
    #[altium_format_derive::altium_enum]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum TestExplicitValues {
        #[altium(value = 10)]
        First,
        #[altium(value = 20)]
        Second,
        #[altium(value = 30)]
        Third,
    }

    #[test]
    fn enum_explicit_values() {
        use crate::v2::traits::AltiumEnum;
        assert_eq!(TestExplicitValues::from_int(10), TestExplicitValues::First);
        assert_eq!(TestExplicitValues::from_int(20), TestExplicitValues::Second);
        assert_eq!(TestExplicitValues::from_int(30), TestExplicitValues::Third);
        assert_eq!(TestExplicitValues::First.to_int(), 10);
        assert_eq!(TestExplicitValues::Second.to_int(), 20);
    }

    // -----------------------------------------------------------------------
    // altium_record tests (param-based)
    // -----------------------------------------------------------------------

    #[altium_format_derive::altium_record(kind = "sch", record_id = 99, codec = "params")]
    struct TestRecord {
        #[altium(key = "NAME")]
        name: String,

        #[altium(key = "VALUE")]
        value: i32,
    }

    #[test]
    fn record_struct_replaced_with_origin() {
        // Verify the struct only has an `origin` field (backing store).
        use crate::v2::backing_store::{ParamOrigin, RecordOrigin};
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=99|NAME=Test|VALUE=42|"));
        let rec = TestRecord::from_origin(origin);
        // If struct had extra fields this wouldn't compile.
        let _origin_ref = &rec.origin;
    }

    #[test]
    fn record_getter_reads_from_backing_store() {
        use crate::v2::backing_store::{ParamOrigin, RecordOrigin};
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=99|NAME=Test|VALUE=42|"));
        let rec = TestRecord::from_origin(origin);
        assert_eq!(rec.name(), "Test");
        assert_eq!(rec.value(), 42);
    }

    #[test]
    fn record_try_getter() {
        use crate::v2::backing_store::{ParamOrigin, RecordOrigin};
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=99|NAME=Hello|"));
        let rec = TestRecord::from_origin(origin);
        assert_eq!(rec.try_name(), Some("Hello".to_string()));
        // VALUE is missing, try_getter should return None
        assert_eq!(rec.try_value(), None);
    }

    #[test]
    fn record_setter_writes_to_backing_store() {
        use crate::v2::backing_store::{ParamOrigin, RecordOrigin};
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=99|NAME=Test|VALUE=42|"));
        let mut rec = TestRecord::from_origin(origin);
        rec.set_name("Updated".to_string());
        assert_eq!(rec.name(), "Updated");
        rec.set_value(100);
        assert_eq!(rec.value(), 100);
    }

    #[test]
    fn record_update_closure() {
        use crate::v2::backing_store::{ParamOrigin, RecordOrigin};
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=99|NAME=Test|VALUE=42|"));
        let mut rec = TestRecord::from_origin(origin);
        let old_name = rec.update_name(|n| {
            let old = n.clone();
            *n = "Modified".to_string();
            old
        });
        assert_eq!(old_name, "Test");
        assert_eq!(rec.name(), "Modified");
    }

    #[test]
    fn record_builder() {
        use crate::v2::backing_store::{ParamOrigin, RecordOrigin};
        fn template() -> RecordOrigin {
            RecordOrigin::Param(ParamOrigin::new("|RECORD=99|"))
        }
        let rec = TestRecord::builder(template)
            .name("Built".to_string())
            .value(77)
            .build();
        assert_eq!(rec.name(), "Built");
        assert_eq!(rec.value(), 77);
    }

    #[test]
    fn record_type_trait() {
        use crate::v2::traits::RecordType;
        assert_eq!(TestRecord::RECORD_ID, 99);

        use crate::v2::backing_store::{ParamOrigin, RecordOrigin};
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=99|NAME=X|"));
        let rec = TestRecord::from_origin(origin);
        // Verify origin() returns reference to the backing store
        let _origin = rec.origin();
    }

    #[test]
    fn record_getter_default_for_missing_key() {
        use crate::v2::backing_store::{ParamOrigin, RecordOrigin};
        // Create a record with only RECORD key, no NAME or VALUE
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=99|"));
        let rec = TestRecord::from_origin(origin);
        // Getters should return default values for missing keys
        assert_eq!(rec.name(), ""); // String::default()
        assert_eq!(rec.value(), 0); // i32::default()
    }

    // -----------------------------------------------------------------------
    // altium_record tests (param-based with skip)
    // -----------------------------------------------------------------------

    #[altium_format_derive::altium_record(kind = "sch", record_id = 98, codec = "params")]
    struct TestRecordWithSkip {
        #[altium(key = "VISIBLE")]
        visible: bool,

        #[altium(skip)]
        _internal: i32,
    }

    #[test]
    fn record_skip_field() {
        use crate::v2::backing_store::{ParamOrigin, RecordOrigin};
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=98|VISIBLE=T|"));
        let rec = TestRecordWithSkip::from_origin(origin);
        assert!(rec.visible());
        // _internal should have no getter/setter generated
    }
}
