#[derive(Clone, Debug, Default)]
pub struct Unique<T> {
    pub inner: T,
}

impl<T> Unique<T> {
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }
}
