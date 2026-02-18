mod kind;

pub use kind::*;

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use smallvec::SmallVec;

use crate::{PrimaryKey, Unique};

#[derive(Clone, Debug, PartialEq)]
pub enum Datatype {
    Int(i32),
    BigInt(i64),

    Float(f32),
    Double(f64),

    Text(String),

    Blob(Vec<u8>),

    Bool(bool),

    Null,
}

impl<D: Into<Datatype>> Into<Datatype> for Option<D> {
    fn into(self) -> Datatype {
        match self {
            Some(datatype) => datatype.into(),
            None => Datatype::Null,
        }
    }
}

impl<D: Into<Datatype>> Into<Datatype> for PrimaryKey<D> {
    fn into(self) -> Datatype {
        self.inner.into()
    }
}

impl<D: Into<Datatype>> Into<Datatype> for Unique<D> {
    fn into(self) -> Datatype {
        self.inner.into()
    }
}

impl Into<Datatype> for i32 {
    fn into(self) -> Datatype {
        Datatype::Int(self)
    }
}

impl Into<Datatype> for i64 {
    fn into(self) -> Datatype {
        Datatype::BigInt(self)
    }
}

impl Into<Datatype> for f32 {
    fn into(self) -> Datatype {
        Datatype::Float(self)
    }
}

impl Into<Datatype> for f64 {
    fn into(self) -> Datatype {
        Datatype::Double(self)
    }
}

impl Into<Datatype> for bool {
    fn into(self) -> Datatype {
        Datatype::Bool(self)
    }
}

impl Into<Datatype> for String {
    fn into(self) -> Datatype {
        Datatype::Text(self)
    }
}

impl Into<Datatype> for &str {
    fn into(self) -> Datatype {
        Datatype::Text(self.to_string())
    }
}

#[derive(Debug)]
pub enum DatatypeConversionError {
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },
    UnexpectedNull,
    WrongNumberOfValues {
        expected: usize,
        got: usize,
    },
}

impl std::fmt::Display for DatatypeConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeMismatch { expected, got } => {
                write!(f, "type mismatch: expected {expected}, got {got}")
            }
            Self::UnexpectedNull => write!(f, "unexpected null value"),
            Self::WrongNumberOfValues { expected, got } => {
                write!(f, "wrong number of values: expected {expected}, got {got}")
            }
        }
    }
}

impl std::error::Error for DatatypeConversionError {}

impl Datatype {
    fn discriminant(&self) -> u8 {
        match self {
            Datatype::Null => 0,
            Datatype::Bool(_) => 1,
            Datatype::Int(_) => 2,
            Datatype::BigInt(_) => 3,
            Datatype::Float(_) => 4,
            Datatype::Double(_) => 5,
            Datatype::Text(_) => 6,
            Datatype::Blob(_) => 7,
        }
    }
}

impl Eq for Datatype {}

impl Hash for Datatype {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.discriminant().hash(state);
        match self {
            Datatype::Int(v) => v.hash(state),
            Datatype::BigInt(v) => v.hash(state),
            Datatype::Float(v) => v.to_bits().hash(state),
            Datatype::Double(v) => v.to_bits().hash(state),
            Datatype::Text(v) => v.hash(state),
            Datatype::Blob(v) => v.hash(state),
            Datatype::Bool(v) => v.hash(state),
            Datatype::Null => {}
        }
    }
}

impl PartialOrd for Datatype {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Datatype {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Datatype::Int(a), Datatype::Int(b)) => a.cmp(b),
            (Datatype::Int(a), Datatype::BigInt(b)) => (*a as i64).cmp(b),
            (Datatype::BigInt(a), Datatype::Int(b)) => a.cmp(&(*b as i64)),
            (Datatype::BigInt(a), Datatype::BigInt(b)) => a.cmp(b),
            (Datatype::Float(a), Datatype::Float(b)) => a.total_cmp(b),
            (Datatype::Float(a), Datatype::Double(b)) => (*a as f64).total_cmp(b),
            (Datatype::Double(a), Datatype::Float(b)) => a.total_cmp(&(*b as f64)),
            (Datatype::Double(a), Datatype::Double(b)) => a.total_cmp(b),
            (Datatype::Text(a), Datatype::Text(b)) => a.cmp(b),
            (Datatype::Blob(a), Datatype::Blob(b)) => a.cmp(b),
            (Datatype::Bool(a), Datatype::Bool(b)) => a.cmp(b),
            (Datatype::Null, Datatype::Null) => Ordering::Equal,
            _ => self.discriminant().cmp(&other.discriminant()),
        }
    }
}

/// An order key extracted from ORDER BY columns in a query result.
/// Used by `OrderedMap` to maintain sorted iteration order.
///
/// Each component has an associated direction flag. When `reversed[i]` is true,
/// the comparison for that component is reversed (for ORDER BY ... DESC).
#[derive(Clone, Debug)]
pub struct OrderKey {
    pub values: SmallVec<[Datatype; 1]>,
    pub reversed: SmallVec<[bool; 1]>,
}

impl Default for OrderKey {
    fn default() -> Self {
        Self {
            values: SmallVec::new(),
            reversed: SmallVec::new(),
        }
    }
}

impl OrderKey {
    pub fn new(values: SmallVec<[Datatype; 1]>, reversed: SmallVec<[bool; 1]>) -> Self {
        Self { values, reversed }
    }

    /// Construct an all-ascending OrderKey (backwards compatible).
    pub fn asc(values: SmallVec<[Datatype; 1]>) -> Self {
        let len = values.len();
        Self {
            values,
            reversed: smallvec::smallvec![false; len],
        }
    }
}

impl PartialEq for OrderKey {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values
    }
}

impl Eq for OrderKey {}

impl Hash for OrderKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.values.hash(state);
    }
}

impl PartialOrd for OrderKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderKey {
    fn cmp(&self, other: &Self) -> Ordering {
        for (i, (a, b)) in self.values.iter().zip(other.values.iter()).enumerate() {
            let cmp = a.cmp(b);
            if cmp != Ordering::Equal {
                let is_reversed = self.reversed.get(i).copied().unwrap_or(false);
                return if is_reversed { cmp.reverse() } else { cmp };
            }
        }
        self.values.len().cmp(&other.values.len())
    }
}

impl std::fmt::Display for Datatype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Datatype::Int(v) => write!(f, "{v}"),
            Datatype::BigInt(v) => write!(f, "{v}"),
            Datatype::Float(v) => write!(f, "{v}"),
            Datatype::Double(v) => write!(f, "{v}"),
            Datatype::Text(v) => write!(f, "{v}"),
            Datatype::Blob(v) => write!(f, "{v:?}"),
            Datatype::Bool(v) => write!(f, "{v}"),
            Datatype::Null => write!(f, "null"),
        }
    }
}

impl Datatype {
    fn type_name(&self) -> &'static str {
        match self {
            Datatype::Int(_) => "Int",
            Datatype::BigInt(_) => "BigInt",
            Datatype::Float(_) => "Float",
            Datatype::Double(_) => "Double",
            Datatype::Text(_) => "Text",
            Datatype::Blob(_) => "Blob",
            Datatype::Bool(_) => "Bool",
            Datatype::Null => "Null",
        }
    }
}

impl TryFrom<Datatype> for i32 {
    type Error = DatatypeConversionError;

    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        match datatype {
            Datatype::Int(v) => Ok(v),
            Datatype::BigInt(v) => Ok(v as i32),
            other => Err(DatatypeConversionError::TypeMismatch {
                expected: "Int",
                got: other.type_name(),
            }),
        }
    }
}

impl TryFrom<Datatype> for i64 {
    type Error = DatatypeConversionError;

    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        match datatype {
            Datatype::BigInt(v) => Ok(v),
            Datatype::Int(v) => Ok(v as i64),
            other => Err(DatatypeConversionError::TypeMismatch {
                expected: "BigInt",
                got: other.type_name(),
            }),
        }
    }
}

impl TryFrom<Datatype> for f32 {
    type Error = DatatypeConversionError;

    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        match datatype {
            Datatype::Float(v) => Ok(v),
            Datatype::Double(v) => Ok(v as f32),
            other => Err(DatatypeConversionError::TypeMismatch {
                expected: "Float",
                got: other.type_name(),
            }),
        }
    }
}

impl TryFrom<Datatype> for f64 {
    type Error = DatatypeConversionError;

    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        match datatype {
            Datatype::Double(v) => Ok(v),
            Datatype::Float(v) => Ok(v as f64),
            other => Err(DatatypeConversionError::TypeMismatch {
                expected: "Double",
                got: other.type_name(),
            }),
        }
    }
}

impl TryFrom<Datatype> for bool {
    type Error = DatatypeConversionError;

    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        match datatype {
            Datatype::Bool(v) => Ok(v),
            Datatype::Int(v) => Ok(v != 0),
            Datatype::BigInt(v) => Ok(v != 0),
            other => Err(DatatypeConversionError::TypeMismatch {
                expected: "Bool",
                got: other.type_name(),
            }),
        }
    }
}

impl TryFrom<Datatype> for String {
    type Error = DatatypeConversionError;

    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        match datatype {
            Datatype::Text(v) => Ok(v),
            other => Err(DatatypeConversionError::TypeMismatch {
                expected: "Text",
                got: other.type_name(),
            }),
        }
    }
}

impl TryFrom<Datatype> for Vec<u8> {
    type Error = DatatypeConversionError;

    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        match datatype {
            Datatype::Blob(v) => Ok(v),
            other => Err(DatatypeConversionError::TypeMismatch {
                expected: "Blob",
                got: other.type_name(),
            }),
        }
    }
}

impl<T: TryFrom<Datatype, Error = DatatypeConversionError>> TryFrom<Datatype> for Option<T> {
    type Error = DatatypeConversionError;

    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        match datatype {
            Datatype::Null => Ok(None),
            other => Ok(Some(T::try_from(other)?)),
        }
    }
}

impl<T: TryFrom<Datatype, Error = DatatypeConversionError>> TryFrom<Datatype> for PrimaryKey<T> {
    type Error = DatatypeConversionError;

    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        Ok(PrimaryKey::new(T::try_from(datatype)?))
    }
}

impl<T: TryFrom<Datatype, Error = DatatypeConversionError>> TryFrom<Datatype> for Unique<T> {
    type Error = DatatypeConversionError;

    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        Ok(Unique::new(T::try_from(datatype)?))
    }
}
