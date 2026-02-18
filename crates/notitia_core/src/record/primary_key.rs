use std::ops::Deref;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PrimaryKey<T> {
    pub(crate) inner: T,
}

impl<T> PrimaryKey<T> {
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Deref for PrimaryKey<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T: std::fmt::Display> std::fmt::Display for PrimaryKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
