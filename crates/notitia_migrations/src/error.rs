#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
