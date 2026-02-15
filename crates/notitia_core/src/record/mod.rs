mod primary_key;
use std::sync::LazyLock;

pub use primary_key::PrimaryKey;

mod unique;
pub use unique::Unique;

use crate::{Datatype, DatatypeKind, FieldExpr, FieldKind};

pub type FieldsDef = LazyLock<Box<[(&'static str, DatatypeKind)]>>;
pub type FieldsDefArray = Box<[(&'static str, DatatypeKind)]>;

pub trait Record: Clone {
    type FieldKind: FieldKind;

    const _FIELDS: FieldsDef;

    fn into_datatypes(self) -> Vec<(&'static str, Datatype)>;
}

#[derive(Clone)]
pub struct UnsetField;

pub trait BuiltRecord {
    type Record;
    fn finish(self) -> Self::Record;
}

pub trait PartialRecord: Clone {
    type FieldKind: FieldKind;
    fn into_set_fields(self) -> Vec<(&'static str, FieldExpr)>;
}

/// Trait for field storage in the builder type-state pattern.
/// `UnsetField` returns `None`, `FieldExpr` returns `Some(expr)`.
pub trait MaybeSetExpr: Clone {
    fn into_field_expr(self) -> Option<FieldExpr>;
}

impl MaybeSetExpr for UnsetField {
    fn into_field_expr(self) -> Option<FieldExpr> {
        None
    }
}

impl MaybeSetExpr for FieldExpr {
    fn into_field_expr(self) -> Option<FieldExpr> {
        Some(self)
    }
}

// Keep the old MaybeSet trait for BuiltRecord::finish (which still needs concrete types).
pub trait MaybeSet: Clone {
    fn into_datatype(self) -> Option<Datatype>;
}

impl MaybeSet for UnsetField {
    fn into_datatype(self) -> Option<Datatype> {
        None
    }
}

impl<T: Into<Datatype> + Clone> MaybeSet for T {
    fn into_datatype(self) -> Option<Datatype> {
        Some(self.into())
    }
}
