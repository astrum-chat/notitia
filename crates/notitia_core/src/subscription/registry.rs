use std::sync::Mutex;

use super::{MutationEvent, SubscriptionDescriptor};

pub struct SubscriptionRegistry {
    subscribers: Mutex<Vec<SubscriberEntry>>,
}

struct SubscriberEntry {
    descriptor: SubscriptionDescriptor,
    /// Type-erased callback. Returns `false` if the subscriber is dead (channel disconnected).
    notify: Box<dyn Fn(&MutationEvent) -> bool + Send + Sync>,
}

impl SubscriptionRegistry {
    pub fn new() -> Self {
        Self {
            subscribers: Mutex::new(Vec::new()),
        }
    }

    pub fn register(
        &self,
        descriptor: SubscriptionDescriptor,
        notify: Box<dyn Fn(&MutationEvent) -> bool + Send + Sync>,
    ) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.push(SubscriberEntry { descriptor, notify });
    }

    /// Broadcast a mutation event to all matching subscribers.
    /// Removes any subscribers whose channels have been dropped.
    pub fn broadcast(&self, event: &MutationEvent) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.retain(|entry| {
            if !super::overlap::event_matches_descriptor(event, &entry.descriptor) {
                return true; // not relevant, but still alive
            }
            (entry.notify)(event) // returns false if channel disconnected
        });
    }
}
