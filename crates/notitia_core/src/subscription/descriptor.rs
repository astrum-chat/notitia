use smallvec::SmallVec;

use crate::FieldFilter;

#[derive(Clone, Debug)]
pub struct SubscriptionDescriptor {
    pub tables: SmallVec<[&'static str; 2]>,
    pub field_names: SmallVec<[&'static str; 4]>,
    pub filters: SmallVec<[FieldFilter; 1]>,
}
