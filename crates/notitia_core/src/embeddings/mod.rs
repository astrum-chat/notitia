use crate::{
    Datatype, DatatypeConversionError, EmbeddedTableDef, FieldExpr, FieldFilter, MutationEvent,
    MutationEventKind, MutationHook,
};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zvec_bindings::{
    CollectionSchema, Doc, IndexParams, MetricType, QuantizeType, SharedCollection, VectorQuery,
    VectorSchema, create_and_open_shared, open_shared,
};

// ---------------------------------------------------------------------------
// Embedded<T> — transparent wrapper for #[db(embed)] fields
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Embedded<T>(pub T);

impl<T> Embedded<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Embedded<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Embedded<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Into<Datatype>> Into<Datatype> for Embedded<T> {
    fn into(self) -> Datatype {
        self.0.into()
    }
}

impl<T: TryFrom<Datatype, Error = DatatypeConversionError>> TryFrom<Datatype> for Embedded<T> {
    type Error = DatatypeConversionError;
    fn try_from(datatype: Datatype) -> Result<Self, Self::Error> {
        Ok(Embedded(T::try_from(datatype)?))
    }
}

impl<T: crate::AsDatatypeKind> crate::AsDatatypeKind for Embedded<T> {
    fn as_datatype_kind() -> crate::DatatypeKind {
        T::as_datatype_kind()
    }
}

// ---------------------------------------------------------------------------
// Metric
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    Cosine,
    L2,
    Ip,
}

impl Metric {
    pub fn from_str(s: &str) -> Self {
        match s {
            "cosine" | "default" => Metric::Cosine,
            "l2" => Metric::L2,
            "ip" => Metric::Ip,
            _ => Metric::Cosine,
        }
    }

    fn to_zvec_metric(self) -> MetricType {
        match self {
            Metric::Cosine => MetricType::Cosine,
            Metric::L2 => MetricType::L2,
            Metric::Ip => MetricType::Ip,
        }
    }
}

// ---------------------------------------------------------------------------
// EmbeddingFieldDef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EmbeddingFieldDef {
    pub field_name: &'static str,
    pub metric: Metric,
}

impl EmbeddingFieldDef {
    pub fn from_raw(field_name: &'static str, metric_str: &'static str) -> Self {
        Self {
            field_name,
            metric: Metric::from_str(metric_str),
        }
    }
}

// ---------------------------------------------------------------------------
// DatabaseEmbedder trait
// ---------------------------------------------------------------------------

pub trait DatabaseEmbedder: Send + Sync {
    fn embed(&self, text: &str) -> Vec<f32>;
    fn dimension(&self) -> u32;
}

impl DatabaseEmbedder for Box<dyn DatabaseEmbedder> {
    fn embed(&self, text: &str) -> Vec<f32> {
        (**self).embed(text)
    }
    fn dimension(&self) -> u32 {
        (**self).dimension()
    }
}

// ---------------------------------------------------------------------------
// Embedding — input type for similarity search queries
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum Embedding {
    Text(String),
    Vector(Vec<f32>),
}

impl From<&str> for Embedding {
    fn from(s: &str) -> Self {
        Embedding::Text(s.to_string())
    }
}

impl From<String> for Embedding {
    fn from(s: String) -> Self {
        Embedding::Text(s)
    }
}

impl From<Vec<f32>> for Embedding {
    fn from(v: Vec<f32>) -> Self {
        Embedding::Vector(v)
    }
}

// ---------------------------------------------------------------------------
// EmbeddingError
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("unknown table: {0}")]
    UnknownTable(String),
    #[error("unknown embedded field: {0}")]
    UnknownField(String),
    #[error("zvec error: {0}")]
    Zvec(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("field '{field}' is not text")]
    NotText { field: &'static str },
}

impl From<zvec_bindings::Error> for EmbeddingError {
    fn from(e: zvec_bindings::Error) -> Self {
        Self::Zvec(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// SimilarityResult
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SimilarityResult {
    pub pk: String,
    pub score: f32,
}

// ---------------------------------------------------------------------------
// EmbeddingSidecar
// ---------------------------------------------------------------------------

struct TableEmbeddingState {
    collection: SharedCollection,
    fields: Vec<EmbeddingFieldDef>,
    pk_field: &'static str,
}

fn vector_field_name(field: &str) -> String {
    format!("{field}_embedding")
}

pub struct EmbeddingSidecar<E: DatabaseEmbedder> {
    embedder: E,
    base_dir: PathBuf,
    tables: HashMap<&'static str, TableEmbeddingState>,
}

impl<E: DatabaseEmbedder> EmbeddingSidecar<E> {
    pub fn new(db_path: &str, embedder: E) -> Result<Self, EmbeddingError> {
        let raw = db_path.strip_prefix("sqlite:").unwrap_or(db_path);
        let path = Path::new(raw);
        let parent = path.parent().unwrap_or(Path::new("."));
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("db");
        let base_dir = parent.join(format!("{stem}_embeddings"));
        Self::new_with_path(base_dir, embedder)
    }

    pub fn new_with_path(path: impl AsRef<Path>, embedder: E) -> Result<Self, EmbeddingError> {
        let base_dir = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_dir)?;
        Ok(Self {
            embedder,
            base_dir,
            tables: HashMap::new(),
        })
    }

    pub fn register_table(
        &mut self,
        table_name: &'static str,
        embedded_fields: &[(&'static str, &'static str)],
        pk_field: &'static str,
    ) -> Result<(), EmbeddingError> {
        let fields: Vec<EmbeddingFieldDef> = embedded_fields
            .iter()
            .map(|(name, metric)| EmbeddingFieldDef::from_raw(name, metric))
            .collect();

        let dim = self.embedder.dimension();
        let table_dir = self.base_dir.join(table_name);
        let table_path = table_dir.to_str().unwrap_or(".");

        let collection = if table_dir.exists() {
            open_shared(table_path)?
        } else {
            let mut schema = CollectionSchema::new(table_name);
            for field in &fields {
                let vname = vector_field_name(field.field_name);
                schema
                    .add_field(VectorSchema::fp32(&vname, dim).into())
                    .map_err(zvec_bindings::Error::from)?;
            }
            create_and_open_shared(table_path, schema)?
        };

        for field in &fields {
            let vname = vector_field_name(field.field_name);
            let params = IndexParams::hnsw(
                16,
                200,
                field.metric.to_zvec_metric(),
                QuantizeType::Undefined,
            );
            let _ = collection.create_index(&vname, params);
        }

        self.tables.insert(
            table_name,
            TableEmbeddingState {
                collection,
                fields,
                pk_field,
            },
        );

        Ok(())
    }

    pub fn on_insert(
        &self,
        table_name: &'static str,
        values: &[(&str, Datatype)],
    ) -> Result<(), EmbeddingError> {
        let state = self
            .tables
            .get(table_name)
            .ok_or_else(|| EmbeddingError::UnknownTable(table_name.to_string()))?;

        let pk = values
            .iter()
            .find(|(name, _)| *name == state.pk_field)
            .map(|(_, v)| v.to_string())
            .ok_or_else(|| EmbeddingError::UnknownField(state.pk_field.to_string()))?;

        let mut doc = Doc::id(&pk);

        for field in &state.fields {
            let text = values
                .iter()
                .find(|(name, _)| *name == field.field_name)
                .and_then(|(_, v)| match v {
                    Datatype::Text(s) => Some(s.as_str()),
                    _ => None,
                })
                .ok_or(EmbeddingError::NotText {
                    field: field.field_name,
                })?;

            let vector = self.embedder.embed(text);
            let vname = vector_field_name(field.field_name);
            doc.set_vector(&vname, &vector)?;
        }

        state.collection.insert(&[doc])?;
        Ok(())
    }

    pub fn on_update(
        &self,
        table_name: &'static str,
        pk: &str,
        changed_fields: &[(&str, &str)],
    ) -> Result<(), EmbeddingError> {
        let state = self
            .tables
            .get(table_name)
            .ok_or_else(|| EmbeddingError::UnknownTable(table_name.to_string()))?;

        let mut doc = Doc::id(pk);

        for (field_name, text) in changed_fields {
            let field = state
                .fields
                .iter()
                .find(|f| f.field_name == *field_name)
                .ok_or_else(|| EmbeddingError::UnknownField(field_name.to_string()))?;

            let vector = self.embedder.embed(text);
            let vname = vector_field_name(field.field_name);
            doc.set_vector(&vname, &vector)?;
        }

        state.collection.upsert(&[doc])?;
        Ok(())
    }

    pub fn on_delete(&self, table_name: &'static str, pk: &str) -> Result<(), EmbeddingError> {
        let state = self
            .tables
            .get(table_name)
            .ok_or_else(|| EmbeddingError::UnknownTable(table_name.to_string()))?;

        state.collection.delete(&[pk])?;
        Ok(())
    }

    pub fn similarity_search(
        &self,
        table_name: &'static str,
        field: &str,
        query: &str,
        topk: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError> {
        let query_vec = self.embedder.embed(query);
        self.similarity_search_vec(table_name, field, &query_vec, topk)
    }

    pub fn similarity_search_vec(
        &self,
        table_name: &'static str,
        field: &str,
        query_vec: &[f32],
        topk: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError> {
        let state = self
            .tables
            .get(table_name)
            .ok_or_else(|| EmbeddingError::UnknownTable(table_name.to_string()))?;

        if !state.fields.iter().any(|f| f.field_name == field) {
            return Err(EmbeddingError::UnknownField(field.to_string()));
        }

        let vname = vector_field_name(field);
        let vq = VectorQuery::new(&vname).topk(topk).vector(query_vec)?;
        let results = state.collection.query(vq)?;

        let mut out = Vec::with_capacity(results.len());
        for doc in results.iter() {
            out.push(SimilarityResult {
                pk: doc.pk().to_string(),
                score: doc.score(),
            });
        }

        Ok(out)
    }

    pub fn embed(&self, text: &str) -> Vec<f32> {
        self.embedder.embed(text)
    }
}

// ---------------------------------------------------------------------------
// DynEmbeddingSidecar — object-safe trait for type-erasing the embedder
// ---------------------------------------------------------------------------

trait DynEmbeddingSidecar: Send + Sync {
    fn on_insert(
        &self,
        table_name: &'static str,
        values: &[(&str, Datatype)],
    ) -> Result<(), EmbeddingError>;
    fn on_update(
        &self,
        table_name: &'static str,
        pk: &str,
        changed: &[(&str, &str)],
    ) -> Result<(), EmbeddingError>;
    fn on_delete(&self, table_name: &'static str, pk: &str) -> Result<(), EmbeddingError>;
    fn has_table(&self, table_name: &str) -> bool;
    fn table_pk_field(&self, table_name: &str) -> Option<&'static str>;
    fn table_embedded_field_names(&self, table_name: &str) -> Vec<&'static str>;
    fn similarity_search(
        &self,
        table_name: &'static str,
        field: &str,
        query: &str,
        topk: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError>;
    fn similarity_search_vec(
        &self,
        table_name: &'static str,
        field: &str,
        query_vec: &[f32],
        topk: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError>;
    fn embed(&self, text: &str) -> Vec<f32>;
}

impl<E: DatabaseEmbedder + Send + Sync> DynEmbeddingSidecar for EmbeddingSidecar<E> {
    fn on_insert(
        &self,
        table_name: &'static str,
        values: &[(&str, Datatype)],
    ) -> Result<(), EmbeddingError> {
        self.on_insert(table_name, values)
    }

    fn on_update(
        &self,
        table_name: &'static str,
        pk: &str,
        changed: &[(&str, &str)],
    ) -> Result<(), EmbeddingError> {
        self.on_update(table_name, pk, changed)
    }

    fn on_delete(&self, table_name: &'static str, pk: &str) -> Result<(), EmbeddingError> {
        self.on_delete(table_name, pk)
    }

    fn has_table(&self, table_name: &str) -> bool {
        self.tables.contains_key(table_name)
    }

    fn table_pk_field(&self, table_name: &str) -> Option<&'static str> {
        self.tables.get(table_name).map(|s| s.pk_field)
    }

    fn table_embedded_field_names(&self, table_name: &str) -> Vec<&'static str> {
        self.tables
            .get(table_name)
            .map(|s| s.fields.iter().map(|f| f.field_name).collect())
            .unwrap_or_default()
    }

    fn similarity_search(
        &self,
        table_name: &'static str,
        field: &str,
        query: &str,
        topk: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError> {
        self.similarity_search(table_name, field, query, topk)
    }

    fn similarity_search_vec(
        &self,
        table_name: &'static str,
        field: &str,
        query_vec: &[f32],
        topk: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError> {
        self.similarity_search_vec(table_name, field, query_vec, topk)
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        self.embed(text)
    }
}

// ---------------------------------------------------------------------------
// EmbeddingManager — non-generic wrapper stored inside Notitia
// ---------------------------------------------------------------------------

pub struct EmbeddingManager {
    inner: Mutex<Box<dyn DynEmbeddingSidecar>>,
}

impl EmbeddingManager {
    pub fn new<E: DatabaseEmbedder + Send + Sync + 'static>(
        embeddings_uri: &str,
        embedder: E,
        tables: &[EmbeddedTableDef],
    ) -> Result<Self, EmbeddingError> {
        let mut sidecar = EmbeddingSidecar::new_with_path(embeddings_uri, embedder)?;
        for def in tables {
            sidecar.register_table(def.table_name, def.embedded_fields, def.pk_field)?;
        }
        Ok(Self {
            inner: Mutex::new(Box::new(sidecar)),
        })
    }

    pub fn similarity_search(
        &self,
        table_name: &'static str,
        field: &str,
        query: &str,
        topk: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError> {
        self.inner
            .lock()
            .unwrap()
            .similarity_search(table_name, field, query, topk)
    }

    pub fn similarity_search_vec(
        &self,
        table_name: &'static str,
        field: &str,
        query_vec: &[f32],
        topk: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError> {
        self.inner
            .lock()
            .unwrap()
            .similarity_search_vec(table_name, field, query_vec, topk)
    }

    pub fn pk_field_for_table(&self, table_name: &str) -> Option<&'static str> {
        self.inner.lock().unwrap().table_pk_field(table_name)
    }

    pub fn embed(&self, text: &str) -> Vec<f32> {
        self.inner.lock().unwrap().embed(text)
    }

    fn extract_pk(
        sidecar: &dyn DynEmbeddingSidecar,
        table_name: &str,
        filters: &[FieldFilter],
    ) -> Option<String> {
        let pk_field = sidecar.table_pk_field(table_name)?;
        filters.iter().find_map(|f| {
            if let FieldFilter::Eq(meta) = f {
                if meta.left.field_name == pk_field {
                    return Some(meta.right.to_string());
                }
            }
            None
        })
    }
}

impl MutationHook for EmbeddingManager {
    fn on_event(&self, event: &MutationEvent) {
        let inner = self.inner.lock().unwrap();
        if !inner.has_table(event.table_name) {
            return;
        }

        match &event.kind {
            MutationEventKind::Insert { values } => {
                let _ = inner.on_insert(event.table_name, values);
            }
            MutationEventKind::Update { changed, filters } => {
                let Some(pk) = Self::extract_pk(&**inner, event.table_name, filters) else {
                    return;
                };

                let embedded_fields = inner.table_embedded_field_names(event.table_name);
                let text_changes: Vec<(&str, &str)> = changed
                    .iter()
                    .filter(|(name, _)| embedded_fields.contains(name))
                    .filter_map(|(name, expr)| {
                        if let FieldExpr::Literal(Datatype::Text(text)) = expr {
                            Some((*name, text.as_str()))
                        } else {
                            None
                        }
                    })
                    .collect();

                if !text_changes.is_empty() {
                    let _ = inner.on_update(event.table_name, &pk, &text_changes);
                }
            }
            MutationEventKind::Delete { filters } => {
                let Some(pk) = Self::extract_pk(&**inner, event.table_name, filters) else {
                    return;
                };
                let _ = inner.on_delete(event.table_name, &pk);
            }
        }
    }
}
