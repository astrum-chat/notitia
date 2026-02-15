use std::{fmt::Debug, marker::PhantomData};

#[macro_export]
macro_rules! union {
    ($ty_a:ty, $ty_b:ty, $($ty_rest:ty),*) => {
        $crate::Union<$ty_a, $crate::Union<$ty_b, union!($($ty_rest),*)>>
    };

    ($ty_a:ty, $ty_b:ty) => {
        $crate::Union<$ty_a, $ty_b>
    };

    ($ty:ty) => {
        $ty
    };
}

/// A trait that only the [Union<L, R>] type implements.
#[allow(private_bounds)]
pub trait IsExplicitUnion: IsExplicitUnionSealed {}
trait IsExplicitUnionSealed {}

/// A trait that every type that can be a union implements.
/// Technically all types implements this trait, since a union
/// of one type is expressed as solely that type.
#[allow(private_bounds)] // `IsUnionSealed` is an internal helper.
pub trait IsUnion: IsUnionSealed {}
trait IsUnionSealed {}

#[derive(Clone, Copy, Debug)]
pub struct Union<L, R>(PhantomData<(L, R)>);
impl<L, R> IsExplicitUnion for Union<L, R> {}
impl<L, R> IsExplicitUnionSealed for Union<L, R> {}

// We need a unique path for each item of
// the union to disambiguiate between impls.
pub trait UnionPath: Clone + Copy + Debug {}

#[derive(Clone, Copy, Debug)]
pub struct UnionRoot;
impl UnionPath for UnionRoot {}

#[derive(Clone, Copy, Debug)]
pub struct UnionLeft<P: UnionPath>(PhantomData<P>);
impl<P: UnionPath> UnionPath for UnionLeft<P> {}

#[derive(Clone, Copy, Debug)]
pub struct UnionRight<P: UnionPath>(PhantomData<P>);
impl<P: UnionPath> UnionPath for UnionRight<P> {}

pub trait IntoUnion<T, P: UnionPath> {}

impl<T> IntoUnion<T, UnionRoot> for T {}

impl<T, LT, RT, P: UnionPath> IntoUnion<Union<LT, RT>, UnionLeft<P>> for T where T: IntoUnion<LT, P> {}
impl<T, LT, RT, P: UnionPath> IntoUnion<Union<LT, RT>, UnionRight<P>> for T where T: IntoUnion<RT, P>
{}

impl<T> IsUnion for T {}
impl<T> IsUnionSealed for T {}
