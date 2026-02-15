use std::sync::{Arc, Mutex, MutexGuard};

use super::SubscriptionMetadata;

pub struct Subscription<T> {
    data: Arc<Mutex<T>>,
    receiver: crossbeam_channel::Receiver<SubscriptionMetadata>,
}

impl<T> Subscription<T> {
    pub(crate) fn new(
        data: Arc<Mutex<T>>,
        receiver: crossbeam_channel::Receiver<SubscriptionMetadata>,
    ) -> Self {
        Self { data, receiver }
    }

    /// Block until the subscription data changes. Returns the metadata
    /// describing what changed.
    pub fn recv(&self) -> Result<SubscriptionMetadata, crossbeam_channel::RecvError> {
        self.receiver.recv()
    }

    /// Returns a reference to the current data.
    pub fn data(&self) -> MutexGuard<'_, T> {
        self.data.lock().unwrap()
    }
}
