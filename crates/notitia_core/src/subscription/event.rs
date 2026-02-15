use smallvec::SmallVec;

use crate::{Datatype, FieldFilter};

#[derive(Clone, Debug)]
pub struct MutationEvent {
    pub table_name: &'static str,
    pub kind: MutationEventKind,
}

#[derive(Clone, Debug)]
pub enum MutationEventKind {
    Insert {
        /// All columns and their values for the inserted row.
        values: Vec<(&'static str, Datatype)>,
    },
    Update {
        /// Only the columns that were set, with new values.
        changed: Vec<(&'static str, Datatype)>,
        /// The filters on the UPDATE statement (which rows were targeted).
        filters: SmallVec<[FieldFilter; 1]>,
    },
    Delete {
        /// The filters on the DELETE statement (which rows were targeted).
        filters: SmallVec<[FieldFilter; 1]>,
    },
}
