#[derive(Debug)]
pub struct ForeignRelationship {
    pub foreign_table: &'static str,
    pub foreign_field: &'static str,
    pub on_delete: OnAction,
    pub on_update: OnAction,
}

impl ForeignRelationship {
    pub const fn new(
        foreign_table: &'static str,
        foreign_field: &'static str,
        on_delete: OnAction,
        on_update: OnAction,
    ) -> Self {
        Self {
            foreign_table,
            foreign_field,
            on_delete,
            on_update,
        }
    }
}

#[derive(Default, Debug)]
pub enum OnAction {
    #[default]
    NoAction,
    Restrict,
    SetNull,
    SetDefault,
    Cascade,
}
