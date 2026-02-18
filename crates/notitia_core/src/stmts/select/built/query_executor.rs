use std::sync::{Arc, Mutex};

use unions::IsUnion;

use crate::{
    Adapter, Database, FieldKindGroup, MutationEvent, Notitia, SubscribableRow, Subscription,
    SubscriptionDescriptor, SubscriptionMetadata, subscription::overlap::event_matches_descriptor,
};

use super::{SelectStmtBuilt, SelectStmtFetchMode};

pub struct QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>
where
    Db: Database,
    Adptr: Adapter,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
    Mode: SelectStmtFetchMode<Fields::Type>,
{
    pub(crate) db: Notitia<Db, Adptr>,
    pub(crate) stmt: SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Mode>,
}

impl<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>
    QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>
where
    Db: Database,
    Adptr: Adapter,
    FieldUnion: IsUnion + Send + Sync,
    FieldPath: Send + Sync,
    Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync,
    Mode: SelectStmtFetchMode<Fields::Type> + Sync,
{
    pub async fn execute(
        #[allow(unused_mut)] mut self,
    ) -> Result<<Mode as SelectStmtFetchMode<Fields::Type>>::Output, Adptr::Error> {
        #[cfg(feature = "embeddings")]
        self.resolve_similarity_search();

        self.stmt.execute(&self.db).await
    }

    #[cfg(feature = "embeddings")]
    fn resolve_similarity_search(&mut self) {
        use crate::{Datatype, Embedding, FieldFilter, FieldFilterInMetadata, TableFieldPair};

        let search = match self.stmt.similarity_search.take() {
            Some(s) => s,
            None => return,
        };

        let mgr = self
            .db
            .embedding_manager()
            .expect("search() used but no EmbeddingManager configured");

        // Resolve Embedding input to a vector
        let query_vec = match &search.query {
            Embedding::Text(text) => mgr.embed(text),
            Embedding::Vector(vec) => vec.clone(),
        };

        // Phase 1: zvec search — get ranked PKs
        let results = mgr
            .similarity_search_vec(
                search.table_name,
                search.field_name,
                &query_vec,
                search.topk,
            )
            .expect("similarity search failed");

        if results.is_empty() {
            // No results — inject an impossible IN filter to return 0 rows
            self.stmt
                .filters
                .push(FieldFilter::In(FieldFilterInMetadata {
                    left: TableFieldPair::new(search.table_name, ""),
                    right: vec![],
                }));
            return;
        }

        // Phase 2: Inject FieldFilter::In for the PK field
        let pk_field = mgr
            .pk_field_for_table(search.table_name)
            .expect("table has no pk field registered in embedding manager");

        let pk_values: Vec<Datatype> = results
            .iter()
            .map(|r| Datatype::Text(r.pk.clone()))
            .collect();

        self.stmt
            .filters
            .push(FieldFilter::In(FieldFilterInMetadata {
                left: TableFieldPair::new(search.table_name, pk_field),
                right: pk_values,
            }));

        // Store PK ordering for CASE-based ORDER BY
        self.stmt.similarity_pk_order = Some(results.iter().map(|r| r.pk.clone()).collect());
    }

    /// Extract the subscription descriptor for this query.
    /// Used by `notitia_gpui` to compare queries and detect changes.
    pub fn descriptor(&self) -> SubscriptionDescriptor {
        SubscriptionDescriptor {
            tables: self.stmt.tables.clone(),
            field_names: self.stmt.fields.field_names(),
            filters: self.stmt.filters.clone(),
            order_by_field_names: self.stmt.order_by.iter().map(|o| o.field).collect(),
            order_by_directions: self
                .stmt
                .order_by
                .iter()
                .map(|o| o.direction.clone())
                .collect(),
        }
    }
}

impl<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>
    QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>
where
    Db: Database,
    Adptr: Adapter,
    FieldUnion: IsUnion + Send + Sync,
    FieldPath: Send + Sync,
    Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync,
    Fields::Type: SubscribableRow,
    Mode: SelectStmtFetchMode<Fields::Type> + Send + Sync + 'static,
    Mode::Output: Clone + PartialEq + Send + 'static,
{
    pub async fn subscribe(self) -> Result<Subscription<Mode::Output>, Adptr::Error> {
        // 1. Execute the query using the mode's own execute method to get initial data.
        let initial_output = self.stmt.execute(&self.db).await?;

        // 2. Build subscription descriptor from the statement.
        let descriptor = SubscriptionDescriptor {
            tables: self.stmt.tables.clone(),
            field_names: self.stmt.fields.field_names(),
            filters: self.stmt.filters.clone(),
            order_by_field_names: self.stmt.order_by.iter().map(|o| o.field).collect(),
            order_by_directions: self
                .stmt
                .order_by
                .iter()
                .map(|o| o.direction.clone())
                .collect(),
        };

        // 3. Create crossbeam channel.
        let (sender, receiver) = crossbeam_channel::unbounded();

        // 4. Store the mode's output in Arc<Mutex<_>> for the Subscription to read.
        let output = Arc::new(Mutex::new(initial_output));

        // 5. Send initial notification.
        let _ = sender.send(SubscriptionMetadata::None);

        // 6. Build the type-erased notify closure.
        //    Uses mode.merge_event() to apply changes directly to the output.
        let notify: Box<dyn Fn(&MutationEvent) -> bool + Send + Sync> = {
            let output = output.clone();
            let descriptor = descriptor.clone();
            let mode = self.stmt.mode;
            Box::new(move |event: &MutationEvent| {
                if !event_matches_descriptor(event, &descriptor) {
                    return true; // still alive, just not relevant
                }

                let mut data = output.lock().unwrap();
                let changed = mode.merge_event(&mut *data, &descriptor, event);

                if !changed {
                    return true;
                }

                drop(data);

                sender
                    .send(SubscriptionMetadata::Changed(event.clone()))
                    .is_ok()
            })
        };

        // 7. Register on the Notitia instance.
        self.db.inner.subscriptions.register(descriptor, notify);

        // 8. Return the subscription handle.
        Ok(Subscription::new(output, receiver))
    }
}
