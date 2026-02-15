mod field_group;
use derivative::Derivative;
pub use field_group::FieldKindGroup;

use std::marker::PhantomData;

use crate::{Database, Datatype, StrongFieldFilter};

pub trait FieldKind: Clone {
    fn name(&self) -> &'static str;
}

pub trait FieldKindOfDatabase<D: Database>: FieldKind {
    fn table_name() -> &'static str;
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct StrongFieldKind<K: FieldKind + Clone, T: Into<Datatype> + Clone> {
    pub kind: K,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _ty: PhantomData<T>,
}

impl<K: FieldKind, T: Into<Datatype> + Clone> StrongFieldKind<K, T> {
    pub const fn new(kind: K) -> Self {
        Self {
            kind,
            _ty: PhantomData,
        }
    }

    pub fn eq(&self, datatype: impl Into<T>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Eq(self.clone(), datatype.into().into())
    }

    pub fn gt(&self, datatype: impl Into<T>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Gt(self.clone(), datatype.into().into())
    }

    pub fn lt(&self, datatype: impl Into<T>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Lt(self.clone(), datatype.into().into())
    }

    pub fn gte(&self, datatype: impl Into<T>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Gte(self.clone(), datatype.into().into())
    }

    pub fn lte(&self, datatype: impl Into<T>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Lte(self.clone(), datatype.into().into())
    }

    pub fn ne(&self, datatype: impl Into<T>) -> StrongFieldFilter<K, T> {
        StrongFieldFilter::Ne(self.clone(), datatype.into().into())
    }
}

pub trait IsStrongFieldKind {
    type Kind: FieldKind;
    type Type: Into<Datatype> + Clone + Send;

    fn name(&self) -> &'static str;
}

impl<K: FieldKind + Clone, T: Into<Datatype> + Clone + Send> IsStrongFieldKind
    for StrongFieldKind<K, T>
{
    type Kind = K;
    type Type = T;

    fn name(&self) -> &'static str {
        self.kind.name()
    }
}
