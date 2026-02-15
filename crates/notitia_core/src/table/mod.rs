use std::{marker::PhantomData, sync::LazyLock};

use derivative::Derivative;

mod table_kind;
pub use table_kind::*;

use crate::{Database, DatatypeKind, Record};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Table<R: Record, Db: Database = ()> {
    pub name: &'static str,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    pub(crate) _record: PhantomData<R>,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    pub(crate) _database: PhantomData<Db>,
}

pub trait IsTable {
    type Record: Record;
    type Database: Database;

    fn name(&self) -> &'static str;
}

impl<R: Record + Clone, Db: Database> IsTable for Table<R, Db> {
    type Record = R;
    type Database = Db;

    fn name(&self) -> &'static str {
        self.name
    }
}

impl<R: Record + Clone, D: Database> Table<R, D> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            _record: PhantomData,
            _database: PhantomData,
        }
    }

    pub fn rows() -> LazyLock<Box<[(&'static str, DatatypeKind)]>> {
        R::_FIELDS
    }

    #[doc(hidden)]
    #[deprecated(note = "`.rows()` should be used instead.")]
    pub fn rows_self(&self) -> LazyLock<Box<[(&'static str, DatatypeKind)]>> {
        R::_FIELDS
    }

    #[doc(hidden)]
    #[deprecated(
        note = "Internal test helper. Do not call in production! This function will panic if invoked."
    )]
    /// Returns the underlying record for testing purposes.
    /// Will panic if called.
    pub fn test_type(&self) -> R {
        unimplemented!()
    }
}
