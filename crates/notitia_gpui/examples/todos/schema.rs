use nanoid::nanoid;
use notitia::{
    AsDatatypeKind, Datatype, DatatypeConversionError, DatatypeKind, DatatypeKindMetadata, Table,
    database, record,
};

#[database]
pub struct TodosDatabase {
    pub todos: Table<Todo>,
}

#[record]
pub struct Todo {
    pub id: UniqueId,
    pub title: String,
    pub content: String,
    pub completed: bool,
}

#[derive(Clone, PartialEq)]
pub struct UniqueId(String);

impl UniqueId {
    pub fn new() -> Self {
        Self(nanoid!())
    }
}

impl AsDatatypeKind for UniqueId {
    fn as_datatype_kind() -> notitia::DatatypeKind {
        DatatypeKind::Text(DatatypeKindMetadata::default())
    }
}

impl Into<Datatype> for UniqueId {
    fn into(self) -> Datatype {
        Datatype::Text(self.0)
    }
}

impl TryFrom<Datatype> for UniqueId {
    type Error = DatatypeConversionError;

    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        String::try_from(datatype).map(UniqueId)
    }
}
