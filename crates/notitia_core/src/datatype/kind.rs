use enum_assoc::Assoc;

use crate::{PrimaryKey, Unique};

#[derive(Debug, Assoc, Clone)]
#[func(pub const fn metadata(&self) -> &DatatypeKindMetadata { _0 })]
#[func(pub const fn metadata_mut(&mut self) -> &mut DatatypeKindMetadata { _0 })]
pub enum DatatypeKind {
    Int(DatatypeKindMetadata),
    BigInt(DatatypeKindMetadata),

    Float(DatatypeKindMetadata),
    Double(DatatypeKindMetadata),

    Text(DatatypeKindMetadata),

    Blob(DatatypeKindMetadata),

    Bool(DatatypeKindMetadata),
}

#[derive(Debug, Default, Clone)]
pub struct DatatypeKindMetadata {
    pub primary_key: bool,
    pub unique: bool,
    pub optional: bool,
}

pub trait AsDatatypeKind {
    fn as_datatype_kind() -> DatatypeKind;
}

impl<T: AsDatatypeKind> AsDatatypeKind for Option<T> {
    fn as_datatype_kind() -> DatatypeKind {
        let mut datatype_kind = T::as_datatype_kind();
        datatype_kind.metadata_mut().optional = true;
        datatype_kind
    }
}

impl<T: AsDatatypeKind + Default> AsDatatypeKind for PrimaryKey<T> {
    fn as_datatype_kind() -> DatatypeKind {
        let mut datatype_kind = T::as_datatype_kind();
        datatype_kind.metadata_mut().primary_key = true;
        datatype_kind
    }
}

impl<T: AsDatatypeKind> AsDatatypeKind for Unique<T> {
    fn as_datatype_kind() -> DatatypeKind {
        let mut datatype_kind = T::as_datatype_kind();
        datatype_kind.metadata_mut().unique = true;
        datatype_kind
    }
}

impl AsDatatypeKind for i32 {
    fn as_datatype_kind() -> DatatypeKind {
        DatatypeKind::Int(DatatypeKindMetadata::default())
    }
}

impl AsDatatypeKind for i64 {
    fn as_datatype_kind() -> DatatypeKind {
        DatatypeKind::BigInt(DatatypeKindMetadata::default())
    }
}

impl AsDatatypeKind for f32 {
    fn as_datatype_kind() -> DatatypeKind {
        DatatypeKind::Float(DatatypeKindMetadata::default())
    }
}

impl AsDatatypeKind for f64 {
    fn as_datatype_kind() -> DatatypeKind {
        DatatypeKind::Double(DatatypeKindMetadata::default())
    }
}

impl AsDatatypeKind for bool {
    fn as_datatype_kind() -> DatatypeKind {
        DatatypeKind::Bool(DatatypeKindMetadata::default())
    }
}

impl AsDatatypeKind for String {
    fn as_datatype_kind() -> DatatypeKind {
        DatatypeKind::Text(DatatypeKindMetadata::default())
    }
}
