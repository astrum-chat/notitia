mod field_group;
use derivative::Derivative;
pub use field_group::FieldKindGroup;

use std::marker::PhantomData;

use crate::{Database, Datatype, FieldExpr, PrimaryKey, StrongFieldFilter, Unique};

#[cfg(feature = "embeddings")]
use crate::Embedded;

/// Maps a field's full type (possibly wrapped) to its inner/filter-comparable type.
///
/// `PrimaryKey<T>`, `Unique<T>`, and `Embedded<T>` all unwrap to `T`.
/// Plain types map to themselves.
pub trait InnerFieldType: Into<Datatype> + Clone {
    type Inner: Into<Datatype> + Clone;
}

impl<T: Into<Datatype> + Clone> InnerFieldType for PrimaryKey<T> {
    type Inner = T;
}

impl<T: Into<Datatype> + Clone> InnerFieldType for Unique<T> {
    type Inner = T;
}

#[cfg(feature = "embeddings")]
impl<T: Into<Datatype> + Clone> InnerFieldType for Embedded<T> {
    type Inner = T;
}

macro_rules! impl_field_wrapper_identity {
    ($($ty:ty),*) => {
        $(impl InnerFieldType for $ty {
            type Inner = $ty;
        })*
    };
}

impl_field_wrapper_identity!(i32, i64, f32, f64, bool, String);

impl<T: InnerFieldType> InnerFieldType for Option<T> {
    type Inner = T::Inner;
}

pub trait FieldKind: Clone {
    fn name(&self) -> &'static str;
}

pub trait FieldKindOfDatabase<D: Database>: FieldKind {
    fn table_name() -> &'static str;
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct StrongFieldKind<K: FieldKind + Clone, T: InnerFieldType> {
    pub kind: K,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _ty: PhantomData<T>,
}

impl<K: FieldKind, T: InnerFieldType> StrongFieldKind<K, T> {
    pub const fn new(kind: K) -> Self {
        Self {
            kind,
            _ty: PhantomData,
        }
    }

    pub fn eq(&self, datatype: impl Into<T::Inner>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Eq(self.clone(), datatype.into().into())
    }

    pub fn gt(&self, datatype: impl Into<T::Inner>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Gt(self.clone(), datatype.into().into())
    }

    pub fn lt(&self, datatype: impl Into<T::Inner>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Lt(self.clone(), datatype.into().into())
    }

    pub fn gte(&self, datatype: impl Into<T::Inner>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Gte(self.clone(), datatype.into().into())
    }

    pub fn lte(&self, datatype: impl Into<T::Inner>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Lte(self.clone(), datatype.into().into())
    }

    pub fn ne(&self, datatype: impl Into<T::Inner>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Ne(self.clone(), datatype.into().into())
    }

    pub fn is_in(
        &self,
        values: impl IntoIterator<Item = impl Into<T::Inner>>,
    ) -> StrongFieldFilter<K, T> {
        let datatypes = values.into_iter().map(|v| v.into().into()).collect();
        StrongFieldFilter::In(self.clone(), datatypes)
    }

    /// Create a concat expression: `Concat(Field(self.name), value)`.
    ///
    /// Used in update builders:
    /// ```ignore
    /// MessageRecord::build().content(MessageRecord::CONTENT.concat("chunk"))
    /// ```
    pub fn concat(&self, value: impl Into<FieldExpr>) -> FieldExpr {
        FieldExpr::Concat(
            Box::new(FieldExpr::Field(self.kind.name())),
            Box::new(value.into()),
        )
    }
}

/// Allow passing a `StrongFieldKind` directly as a `FieldExpr` (becomes `Field` reference).
///
/// ```ignore
/// MessageRecord::build().content(MessageRecord::TITLE)
/// // → Field("title") — sets content to title's current value
/// ```
impl<K: FieldKind, T: InnerFieldType> From<StrongFieldKind<K, T>> for FieldExpr {
    fn from(field: StrongFieldKind<K, T>) -> Self {
        FieldExpr::Field(field.kind.name())
    }
}

pub trait IsStrongFieldKind {
    type Kind: FieldKind;
    type Type: Into<Datatype> + Clone + Send;

    fn name(&self) -> &'static str;
}

impl<K: FieldKind + Clone, T: InnerFieldType + Send> IsStrongFieldKind for StrongFieldKind<K, T> {
    type Kind = K;
    type Type = T;

    fn name(&self) -> &'static str {
        self.kind.name()
    }
}
