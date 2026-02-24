#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Any,
    String,
    Integer,
    Float,
    Bool,
    Dim,
    Coord,
    Color,
    RefExpr,
    Object,
    ObjectArray,
    Selector,
}

#[derive(Debug, Clone, Copy)]
pub struct FieldSchema {
    pub rust_field: &'static str,
    pub name: &'static str,
    pub ty: FieldType,
    pub required: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct OpSchema {
    pub op_name: &'static str,
    pub domain: &'static str,
    pub fields: &'static [FieldSchema],
}

pub trait HasOpsSchema {
    fn ops_schema() -> &'static OpSchema;
}

#[derive(Debug, Clone, Copy)]
pub struct EnumVariantSchema {
    pub canonical: &'static str,
    pub normalized: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct EnumSchema {
    pub name: &'static str,
    pub variants: &'static [EnumVariantSchema],
}

pub trait HasOpsEnum {
    fn ops_enum_schema() -> &'static EnumSchema;
}

pub fn normalize_enum_ident(value: &str) -> String {
    value
        .chars()
        .filter(|ch| *ch != '_')
        .flat_map(char::to_lowercase)
        .collect()
}
