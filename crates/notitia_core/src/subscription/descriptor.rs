use smallvec::SmallVec;

use crate::{FieldFilter, OrderDirection};

#[derive(Clone, Debug, PartialEq)]
pub struct SubscriptionDescriptor {
    pub tables: SmallVec<[&'static str; 2]>,
    pub field_names: SmallVec<[&'static str; 4]>,
    pub filters: SmallVec<[FieldFilter; 1]>,
    pub order_by_field_names: SmallVec<[&'static str; 1]>,
    pub order_by_directions: SmallVec<[OrderDirection; 1]>,
}
