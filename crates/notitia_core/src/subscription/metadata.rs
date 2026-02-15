use super::MutationEvent;

#[derive(Clone, Debug)]
pub enum SubscriptionMetadata {
    None,
    Changed(MutationEvent),
}
