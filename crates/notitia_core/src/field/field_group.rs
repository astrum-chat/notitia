use smallvec::SmallVec;
use unions::{IntoUnion, IsUnion, UnionPath};

use crate::{Datatype, DatatypeConversionError, IsStrongFieldKind, SubscribableRow};

pub trait FieldKindGroup<F, D> {
    type Type: Send;

    fn field_names(&self) -> SmallVec<[&'static str; 4]>;
    fn from_datatypes(
        values: &mut impl Iterator<Item = Datatype>,
    ) -> Result<Self::Type, DatatypeConversionError>;
}

// Single item.
impl<U: IsUnion, P0: UnionPath, F0: IsStrongFieldKind<Kind = impl IntoUnion<U, P0>>>
    FieldKindGroup<U, P0> for F0
where
    F0::Type: TryFrom<Datatype, Error = DatatypeConversionError>,
{
    type Type = F0::Type;

    fn field_names(&self) -> SmallVec<[&'static str; 4]> {
        smallvec::smallvec![self.name()]
    }

    fn from_datatypes(
        values: &mut impl Iterator<Item = Datatype>,
    ) -> Result<Self::Type, DatatypeConversionError> {
        let val = values
            .next()
            .ok_or(DatatypeConversionError::WrongNumberOfValues {
                expected: 1,
                got: 0,
            })?;
        F0::Type::try_from(val)
    }
}

// Array.
impl<U: IsUnion, P0: UnionPath, F0: IsStrongFieldKind<Kind = impl IntoUnion<U, P0>>, const N: usize>
    FieldKindGroup<U, P0> for [F0; N]
where
    F0::Type: TryFrom<Datatype, Error = DatatypeConversionError>,
{
    type Type = [F0::Type; N];

    fn field_names(&self) -> SmallVec<[&'static str; 4]> {
        self.iter().map(|f| f.name()).collect()
    }

    fn from_datatypes(
        values: &mut impl Iterator<Item = Datatype>,
    ) -> Result<Self::Type, DatatypeConversionError> {
        let converted: Vec<F0::Type> = values
            .take(N)
            .map(|v| F0::Type::try_from(v))
            .collect::<Result<Vec<_>, _>>()?;
        converted.try_into().map_err(|v: Vec<F0::Type>| {
            DatatypeConversionError::WrongNumberOfValues {
                expected: N,
                got: v.len(),
            }
        })
    }
}

// Array reference.
impl<
    'a,
    U: IsUnion,
    P0: UnionPath,
    F0: IsStrongFieldKind<Kind = impl IntoUnion<U, P0>>,
    const N: usize,
> FieldKindGroup<U, P0> for &'a [F0; N]
where
    F0::Type: TryFrom<Datatype, Error = DatatypeConversionError>,
{
    type Type = [F0::Type; N];

    fn field_names(&self) -> SmallVec<[&'static str; 4]> {
        self.iter().map(|f| f.name()).collect()
    }

    fn from_datatypes(
        values: &mut impl Iterator<Item = Datatype>,
    ) -> Result<Self::Type, DatatypeConversionError> {
        let converted: Vec<F0::Type> = values
            .take(N)
            .map(|v| F0::Type::try_from(v))
            .collect::<Result<Vec<_>, _>>()?;
        converted.try_into().map_err(|v: Vec<F0::Type>| {
            DatatypeConversionError::WrongNumberOfValues {
                expected: N,
                got: v.len(),
            }
        })
    }
}

// Slice.
// We don't know the length of it so we have to return a boxed slice.
impl<'a, U: IsUnion, P0: UnionPath, F0: IsStrongFieldKind<Kind = impl IntoUnion<U, P0>>>
    FieldKindGroup<U, P0> for &'a [F0]
where
    F0::Type: TryFrom<Datatype, Error = DatatypeConversionError>,
{
    type Type = Box<[F0::Type]>;

    fn field_names(&self) -> SmallVec<[&'static str; 4]> {
        self.iter().map(|f| f.name()).collect()
    }

    fn from_datatypes(
        values: &mut impl Iterator<Item = Datatype>,
    ) -> Result<Self::Type, DatatypeConversionError> {
        values
            .map(|v| F0::Type::try_from(v))
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into_boxed_slice())
    }
}

// Vec.
impl<U: IsUnion, P0: UnionPath, F0: IsStrongFieldKind<Kind = impl IntoUnion<U, P0>>>
    FieldKindGroup<U, P0> for Vec<F0>
where
    F0::Type: TryFrom<Datatype, Error = DatatypeConversionError>,
{
    type Type = Vec<F0::Type>;

    fn field_names(&self) -> SmallVec<[&'static str; 4]> {
        self.iter().map(|f| f.name()).collect()
    }

    fn from_datatypes(
        values: &mut impl Iterator<Item = Datatype>,
    ) -> Result<Self::Type, DatatypeConversionError> {
        values.map(|v| F0::Type::try_from(v)).collect()
    }
}

// Boxed array.
impl<U: IsUnion, P0: UnionPath, F0: IsStrongFieldKind<Kind = impl IntoUnion<U, P0>>, const N: usize>
    FieldKindGroup<U, P0> for Box<[F0; N]>
where
    F0::Type: TryFrom<Datatype, Error = DatatypeConversionError>,
{
    type Type = [F0::Type; N];

    fn field_names(&self) -> SmallVec<[&'static str; 4]> {
        self.iter().map(|f| f.name()).collect()
    }

    fn from_datatypes(
        values: &mut impl Iterator<Item = Datatype>,
    ) -> Result<Self::Type, DatatypeConversionError> {
        let converted: Vec<F0::Type> = values
            .take(N)
            .map(|v| F0::Type::try_from(v))
            .collect::<Result<Vec<_>, _>>()?;
        converted.try_into().map_err(|v: Vec<F0::Type>| {
            DatatypeConversionError::WrongNumberOfValues {
                expected: N,
                got: v.len(),
            }
        })
    }
}

// Boxed slice.
impl<U: IsUnion, P0: UnionPath, F0: IsStrongFieldKind<Kind = impl IntoUnion<U, P0>>>
    FieldKindGroup<U, P0> for Box<[F0]>
where
    F0::Type: TryFrom<Datatype, Error = DatatypeConversionError>,
{
    type Type = Box<[F0::Type]>;

    fn field_names(&self) -> SmallVec<[&'static str; 4]> {
        self.iter().map(|f| f.name()).collect()
    }

    fn from_datatypes(
        values: &mut impl Iterator<Item = Datatype>,
    ) -> Result<Self::Type, DatatypeConversionError> {
        values
            .map(|v| F0::Type::try_from(v))
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into_boxed_slice())
    }
}

macro_rules! impl_field_group {
    (@impl $(($P:ident, $F:ident)),+) => {
        impl<
            U,
            $($P: UnionPath,)+
            $($F: IsStrongFieldKind<Kind = impl IntoUnion<U, $P>>,)+
        > FieldKindGroup<U, ($($P,)+)> for ($($F,)+)
        where
            $($F::Type: TryFrom<Datatype, Error = DatatypeConversionError>,)+
        {
            #[allow(unused)]
            type Type = ($($F::Type),+);

            #[allow(non_snake_case)]
            fn field_names(&self) -> SmallVec<[&'static str; 4]> {
                let ($($F,)+) = self;
                smallvec::smallvec![$($F.name()),+]
            }

            fn from_datatypes(
                values: &mut impl Iterator<Item = Datatype>,
            ) -> Result<Self::Type, DatatypeConversionError> {
                Ok(($({
                    let val = values.next().ok_or(
                        DatatypeConversionError::WrongNumberOfValues { expected: 0, got: 0 },
                    )?;
                    <$F>::Type::try_from(val)?
                }),+))
            }
        }
    };

    (@build [$($acc:tt)*] ($P:ident, $F:ident)) => {
        impl_field_group!(@impl $($acc)* ($P, $F));
    };

    (@build [$($acc:tt)*] ($P:ident, $F:ident), $($rest:tt)+) => {
        impl_field_group!(@impl $($acc)* ($P, $F));
        impl_field_group!(@build [$($acc)* ($P, $F),] $($rest)+);
    };

    ($($P:ident: $F:ident),+ $(,)?) => {
        impl_field_group!(@build [] $(($P, $F)),+);
    };
}

// --- SubscribableRow impls ---

// Single value.
impl<T> SubscribableRow for T
where
    T: Clone
        + PartialEq
        + Into<Datatype>
        + TryFrom<Datatype, Error = DatatypeConversionError>
        + Send
        + 'static,
{
    fn to_datatypes(&self, field_names: &[&'static str]) -> Vec<(&'static str, Datatype)> {
        let name = field_names.first().copied().unwrap_or("");
        vec![(name, self.clone().into())]
    }

    fn from_datatypes(
        values: &mut impl Iterator<Item = Datatype>,
    ) -> Result<Self, DatatypeConversionError> {
        let val = values
            .next()
            .ok_or(DatatypeConversionError::WrongNumberOfValues {
                expected: 1,
                got: 0,
            })?;
        T::try_from(val)
    }
}

// Tuples.
macro_rules! impl_subscribable_row_tuple {
    (@impl $count:expr, $($idx:tt: $T:ident),+) => {
        impl<$($T),+> SubscribableRow for ($($T,)+)
        where
            $($T: Clone + PartialEq + Into<Datatype> + TryFrom<Datatype, Error = DatatypeConversionError> + Send + 'static,)+
        {
            fn to_datatypes(&self, field_names: &[&'static str]) -> Vec<(&'static str, Datatype)> {
                vec![
                    $((
                        field_names.get($idx).copied().unwrap_or(""),
                        self.$idx.clone().into(),
                    )),+
                ]
            }

            fn from_datatypes(
                values: &mut impl Iterator<Item = Datatype>,
            ) -> Result<Self, DatatypeConversionError> {
                Ok(($({
                    let _ = $idx; // use $idx to sequence the expansions
                    let val = values.next().ok_or(
                        DatatypeConversionError::WrongNumberOfValues { expected: $count, got: $idx },
                    )?;
                    $T::try_from(val)?
                },)+))
            }
        }
    };

    (@build [$($acc:tt)*] $idx:tt: $T:ident) => {
        impl_subscribable_row_tuple!(@impl $idx + 1, $($acc)* $idx: $T);
    };

    (@build [$($acc:tt)*] $idx:tt: $T:ident, $($rest:tt)+) => {
        impl_subscribable_row_tuple!(@impl $idx + 1, $($acc)* $idx: $T);
        impl_subscribable_row_tuple!(@build [$($acc)* $idx: $T,] $($rest)+);
    };

    ($($idx:tt: $T:ident),+ $(,)?) => {
        impl_subscribable_row_tuple!(@build [] $($idx: $T),+);
    };
}
// Tier 1: 4 fields (extra_small_fields)
#[cfg(feature = "extra_small_fields")]
impl_field_group!(
    P0: F0,
    P1: F1,
    P2: F2,
    P3: F3,
);

#[cfg(feature = "extra_small_fields")]
impl_subscribable_row_tuple!(
    0: T0,
    1: T1,
    2: T2,
    3: T3,
);

// Tier 2: 12 fields (small_fields)
#[cfg(feature = "small_fields")]
impl_field_group!(
    P0: F0, P1: F1, P2: F2, P3: F3,
    P4: F4, P5: F5, P6: F6, P7: F7,
    P8: F8, P9: F9, P10: F10, P11: F11,
);

#[cfg(feature = "small_fields")]
impl_subscribable_row_tuple!(
    0: T0, 1: T1, 2: T2, 3: T3,
    4: T4, 5: T5, 6: T6, 7: T7,
    8: T8, 9: T9, 10: T10, 11: T11,
);

// Tier 3: 22 fields (medium_fields)
#[cfg(feature = "medium_fields")]
impl_field_group!(
    P0: F0, P1: F1, P2: F2, P3: F3,
    P4: F4, P5: F5, P6: F6, P7: F7,
    P8: F8, P9: F9, P10: F10, P11: F11,
    P12: F12, P13: F13, P14: F14, P15: F15,
    P16: F16, P17: F17, P18: F18, P19: F19,
    P20: F20, P21: F21,
);

#[cfg(feature = "medium_fields")]
impl_subscribable_row_tuple!(
    0: T0, 1: T1, 2: T2, 3: T3,
    4: T4, 5: T5, 6: T6, 7: T7,
    8: T8, 9: T9, 10: T10, 11: T11,
    12: T12, 13: T13, 14: T14, 15: T15,
    16: T16, 17: T17, 18: T18, 19: T19,
    20: T20, 21: T21,
);

// Tier 4: 42 fields (large_fields)
#[cfg(feature = "large_fields")]
impl_field_group!(
    P0: F0, P1: F1, P2: F2, P3: F3,
    P4: F4, P5: F5, P6: F6, P7: F7,
    P8: F8, P9: F9, P10: F10, P11: F11,
    P12: F12, P13: F13, P14: F14, P15: F15,
    P16: F16, P17: F17, P18: F18, P19: F19,
    P20: F20, P21: F21, P22: F22, P23: F23,
    P24: F24, P25: F25, P26: F26, P27: F27,
    P28: F28, P29: F29, P30: F30, P31: F31,
    P32: F32, P33: F33, P34: F34, P35: F35,
    P36: F36, P37: F37, P38: F38, P39: F39,
    P40: F40, P41: F41,
);

#[cfg(feature = "large_fields")]
impl_subscribable_row_tuple!(
    0: T0, 1: T1, 2: T2, 3: T3,
    4: T4, 5: T5, 6: T6, 7: T7,
    8: T8, 9: T9, 10: T10, 11: T11,
    12: T12, 13: T13, 14: T14, 15: T15,
    16: T16, 17: T17, 18: T18, 19: T19,
    20: T20, 21: T21, 22: T22, 23: T23,
    24: T24, 25: T25, 26: T26, 27: T27,
    28: T28, 29: T29, 30: T30, 31: T31,
    32: T32, 33: T33, 34: T34, 35: T35,
    36: T36, 37: T37, 38: T38, 39: T39,
    40: T40, 41: T41,
);

// Tier 5: 64 fields (extra_large_fields)
#[cfg(feature = "extra_large_fields")]
impl_field_group!(
    P0: F0, P1: F1, P2: F2, P3: F3,
    P4: F4, P5: F5, P6: F6, P7: F7,
    P8: F8, P9: F9, P10: F10, P11: F11,
    P12: F12, P13: F13, P14: F14, P15: F15,
    P16: F16, P17: F17, P18: F18, P19: F19,
    P20: F20, P21: F21, P22: F22, P23: F23,
    P24: F24, P25: F25, P26: F26, P27: F27,
    P28: F28, P29: F29, P30: F30, P31: F31,
    P32: F32, P33: F33, P34: F34, P35: F35,
    P36: F36, P37: F37, P38: F38, P39: F39,
    P40: F40, P41: F41, P42: F42, P43: F43,
    P44: F44, P45: F45, P46: F46, P47: F47,
    P48: F48, P49: F49, P50: F50, P51: F51,
    P52: F52, P53: F53, P54: F54, P55: F55,
    P56: F56, P57: F57, P58: F58, P59: F59,
    P60: F60, P61: F61, P62: F62, P63: F63,
);

#[cfg(feature = "extra_large_fields")]
impl_subscribable_row_tuple!(
    0: T0, 1: T1, 2: T2, 3: T3,
    4: T4, 5: T5, 6: T6, 7: T7,
    8: T8, 9: T9, 10: T10, 11: T11,
    12: T12, 13: T13, 14: T14, 15: T15,
    16: T16, 17: T17, 18: T18, 19: T19,
    20: T20, 21: T21, 22: T22, 23: T23,
    24: T24, 25: T25, 26: T26, 27: T27,
    28: T28, 29: T29, 30: T30, 31: T31,
    32: T32, 33: T33, 34: T34, 35: T35,
    36: T36, 37: T37, 38: T38, 39: T39,
    40: T40, 41: T41, 42: T42, 43: T43,
    44: T44, 45: T45, 46: T46, 47: T47,
    48: T48, 49: T49, 50: T50, 51: T51,
    52: T52, 53: T53, 54: T54, 55: T55,
    56: T56, 57: T57, 58: T58, 59: T59,
    60: T60, 61: T61, 62: T62, 63: T63,
);
