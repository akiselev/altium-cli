/// Integration tests for the `#[derive(FromParams)]` macro.
///
/// These tests live inside altium-format (not in altium-format-derive) because
/// `ParameterCollection` is `pub(crate)` — an implementation detail that must
/// not be exposed to external crates.
#[cfg(test)]
mod tests {
    use altium_format_derive::FromParams;
    use altium_format_types::{Coord, CoordPoint};

    use crate::Result;
    use crate::param_collection::ParameterCollection;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn pc(s: &str) -> ParameterCollection {
        let mut data = s.to_owned();
        data.push('\0');
        ParameterCollection::from_bytes(data.as_bytes()).unwrap()
    }

    // ── required field ────────────────────────────────────────────────────────

    #[derive(FromParams, Debug, PartialEq)]
    struct RequiredStruct {
        #[param(key = "NAME")]
        pub name: String,
        #[param(key = "COUNT")]
        pub count: i32,
    }

    #[test]
    fn required_field_present() {
        let mut params = pc("|NAME=hello|COUNT=42|");
        let s = RequiredStruct::from_params(&mut params).unwrap();
        assert_eq!(s.name, "hello");
        assert_eq!(s.count, 42);
    }

    #[test]
    fn required_field_missing_returns_error() {
        let mut params = pc("|NAME=hello|");
        let err = RequiredStruct::from_params(&mut params).unwrap_err();
        assert!(
            matches!(err, crate::AltiumFormatError::MissingParam(_)),
            "expected MissingParam, got {err:?}"
        );
    }

    // ── default field ─────────────────────────────────────────────────────────

    #[derive(FromParams, Debug, PartialEq)]
    struct DefaultStruct {
        #[param(key = "VALUE", default = 99i32)]
        pub value: i32,
        #[param(key = "FLAG", default = false)]
        pub flag: bool,
    }

    #[test]
    fn default_used_when_absent() {
        let mut params = pc("|");
        let s = DefaultStruct::from_params(&mut params).unwrap();
        assert_eq!(s.value, 99);
        assert!(!s.flag);
    }

    #[test]
    fn default_overridden_when_present() {
        let mut params = pc("|VALUE=7|FLAG=T|");
        let s = DefaultStruct::from_params(&mut params).unwrap();
        assert_eq!(s.value, 7);
        assert!(s.flag);
    }

    // ── optional field ────────────────────────────────────────────────────────

    #[derive(FromParams, Debug, PartialEq)]
    struct OptionalStruct {
        #[param(key = "MAYBE", optional)]
        pub maybe: Option<i32>,
    }

    #[test]
    fn optional_returns_none_when_absent() {
        let mut params = pc("|");
        let s = OptionalStruct::from_params(&mut params).unwrap();
        assert_eq!(s.maybe, None);
    }

    #[test]
    fn optional_returns_some_when_present() {
        let mut params = pc("|MAYBE=5|");
        let s = OptionalStruct::from_params(&mut params).unwrap();
        assert_eq!(s.maybe, Some(5));
    }

    // ── coord field ───────────────────────────────────────────────────────────

    #[derive(FromParams, Debug)]
    struct CoordStruct {
        #[param(coord, key = "LOC.X", frac_key = "LOC.X_FRAC")]
        pub x: Coord,
    }

    #[test]
    fn coord_with_frac() {
        let mut params = pc("|LOC.X=100|LOC.X_FRAC=50000|");
        let s = CoordStruct::from_params(&mut params).unwrap();
        assert_eq!(s.x.to_internal(), 100 * 100_000 + 50_000);
    }

    #[test]
    fn coord_without_frac_defaults_to_zero() {
        let mut params = pc("|LOC.X=100|");
        let s = CoordStruct::from_params(&mut params).unwrap();
        assert_eq!(s.x.to_internal(), 100 * 100_000);
    }

    // ── coord_point field ─────────────────────────────────────────────────────

    #[derive(FromParams, Debug)]
    struct CoordPointStruct {
        #[param(
            coord_point,
            x_key = "LX",
            x_frac = "LX_FRAC",
            y_key = "LY",
            y_frac = "LY_FRAC"
        )]
        pub location: CoordPoint,
    }

    #[test]
    fn coord_point_parsed() {
        let mut params = pc("|LX=10|LY=20|");
        let s = CoordPointStruct::from_params(&mut params).unwrap();
        assert_eq!(s.location.x.to_internal(), 10 * 100_000);
        assert_eq!(s.location.y.to_internal(), 20 * 100_000);
    }

    // ── indexed_coords field ──────────────────────────────────────────────────

    #[derive(FromParams, Debug)]
    struct IndexedCoordsStruct {
        #[param(indexed_coords, count_key = "COUNT", x_prefix = "X", y_prefix = "Y")]
        pub vertices: Vec<CoordPoint>,
    }

    #[test]
    fn indexed_coords_parsed() {
        let mut params = pc("|COUNT=2|X1=1|Y1=2|X2=3|Y2=4|");
        let s = IndexedCoordsStruct::from_params(&mut params).unwrap();
        assert_eq!(s.vertices.len(), 2);
        assert_eq!(s.vertices[0].x.to_internal(), 100_000);
        assert_eq!(s.vertices[1].y.to_internal(), 400_000);
    }

    // ── flatten ───────────────────────────────────────────────────────────────

    #[derive(FromParams, Debug, PartialEq)]
    struct BaseStruct {
        #[param(key = "A", default = 0i32)]
        pub a: i32,
    }

    #[derive(FromParams, Debug)]
    struct FlattenStruct {
        #[param(flatten)]
        pub base: BaseStruct,
        #[param(key = "B")]
        pub b: i32,
    }

    #[test]
    fn flatten_composes_base_type() {
        let mut params = pc("|A=7|B=8|");
        let s = FlattenStruct::from_params(&mut params).unwrap();
        assert_eq!(s.base.a, 7);
        assert_eq!(s.b, 8);
    }

    #[test]
    fn flatten_does_not_call_assert_exhausted() {
        // Flatten should leave params that the outer struct will consume.
        let mut params = pc("|A=1|B=2|");
        let s = FlattenStruct::from_params(&mut params).unwrap();
        assert_eq!(s.base.a, 1);
        assert_eq!(s.b, 2);
        // All consumed; should be exhausted now.
        params.assert_exhausted().unwrap();
    }

    // ── list field ────────────────────────────────────────────────────────────

    #[derive(FromParams, Debug)]
    struct ListStruct {
        #[param(list, key = "ITEMS")]
        pub items: Vec<i32>,
    }

    #[test]
    fn list_parsed() {
        let mut params = pc("|ITEMS=1,2,3|");
        let s = ListStruct::from_params(&mut params).unwrap();
        assert_eq!(s.items, vec![1, 2, 3]);
    }

    #[test]
    fn list_missing_returns_error() {
        let mut params = pc("|");
        let err = ListStruct::from_params(&mut params).unwrap_err();
        assert!(matches!(err, crate::AltiumFormatError::MissingParam(_)));
    }

    // ── list_or_empty field ───────────────────────────────────────────────────

    #[derive(FromParams, Debug)]
    struct ListOrEmptyStruct {
        #[param(list_or_empty, key = "ITEMS")]
        pub items: Vec<i32>,
    }

    #[test]
    fn list_or_empty_returns_empty_when_absent() {
        let mut params = pc("|");
        let s = ListOrEmptyStruct::from_params(&mut params).unwrap();
        assert!(s.items.is_empty());
    }

    #[test]
    fn list_or_empty_parsed_when_present() {
        let mut params = pc("|ITEMS=10,20|");
        let s = ListOrEmptyStruct::from_params(&mut params).unwrap();
        assert_eq!(s.items, vec![10, 20]);
    }
}
