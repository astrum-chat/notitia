use std::ops::Deref;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Unique<T> {
    pub inner: T,
}

impl<T> Unique<T> {
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Deref for Unique<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Unique<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
