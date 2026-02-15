#[derive(Clone, Debug, Default)]
pub struct PrimaryKey<T> {
    pub(crate) inner: T,
}

impl<T> PrimaryKey<T> {
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }
}
