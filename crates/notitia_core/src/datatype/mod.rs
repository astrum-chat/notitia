mod kind;

pub use kind::*;

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

impl PartialOrd for Datatype {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Datatype::Int(a), Datatype::Int(b)) => a.partial_cmp(b),
            (Datatype::Int(a), Datatype::BigInt(b)) => (*a as i64).partial_cmp(b),
            (Datatype::BigInt(a), Datatype::Int(b)) => a.partial_cmp(&(*b as i64)),
            (Datatype::BigInt(a), Datatype::BigInt(b)) => a.partial_cmp(b),
            (Datatype::Float(a), Datatype::Float(b)) => a.partial_cmp(b),
            (Datatype::Float(a), Datatype::Double(b)) => (*a as f64).partial_cmp(b),
            (Datatype::Double(a), Datatype::Float(b)) => a.partial_cmp(&(*b as f64)),
            (Datatype::Double(a), Datatype::Double(b)) => a.partial_cmp(b),
            (Datatype::Text(a), Datatype::Text(b)) => a.partial_cmp(b),
            (Datatype::Bool(a), Datatype::Bool(b)) => a.partial_cmp(b),
            _ => None,
        }
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
