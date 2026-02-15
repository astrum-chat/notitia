use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use gpui::{App, AppContext, AsyncApp, ElementId, Entity};
use notitia::{
    Adapter, Database, FieldKindGroup, QueryExecutor, SelectStmtFetchMode, SubscribableRow,
    SubscriptionDescriptor,
};

pub struct DbEntity<T: 'static> {
    entity: Entity<Option<T>>,
}

impl<T: 'static> DbEntity<T> {
    pub fn read<'a>(&self, cx: &'a App) -> Option<&'a T> {
        self.entity.read(cx).as_ref()
    }
}

/// Internal state for a database query subscription.
struct DbQueryState<Output: 'static> {
    /// The actual data entity exposed via DbEntity.
    data_entity: Entity<Option<Output>>,
    /// Flag to signal the bridge thread to stop.
    cancel_flag: Option<Arc<AtomicBool>>,
    /// Descriptor of the current query (for comparison).
    current_descriptor: Option<SubscriptionDescriptor>,
}

pub trait WindowNotitiaExt {
    fn use_keyed_db_query<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>(
        &mut self,
        key: impl Into<ElementId>,
        cx: &mut App,
        init_query: impl FnOnce(
            &mut Self,
            &mut App,
        ) -> QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>,
    ) -> DbEntity<Mode::Output>
    where
        Db: Database + 'static,
        Adptr: Adapter + 'static,
        FieldUnion: unions::IsUnion + Send + Sync + 'static,
        FieldPath: Send + Sync + 'static,
        Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync + 'static,
        Fields::Type: SubscribableRow,
        Mode: SelectStmtFetchMode<Fields::Type> + Send + Sync + 'static,
        Mode::Output: Clone + PartialEq + Send;

    fn use_db_query<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>(
        &mut self,
        cx: &mut App,
        init_query: impl FnOnce(
            &mut Self,
            &mut App,
        ) -> QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>,
    ) -> DbEntity<Mode::Output>
    where
        Db: Database + 'static,
        Adptr: Adapter + 'static,
        FieldUnion: unions::IsUnion + Send + Sync + 'static,
        FieldPath: Send + Sync + 'static,
        Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync + 'static,
        Fields::Type: SubscribableRow,
        Mode: SelectStmtFetchMode<Fields::Type> + Send + Sync + 'static,
        Mode::Output: Clone + PartialEq + Send;
}

impl WindowNotitiaExt for gpui::Window {
    fn use_keyed_db_query<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>(
        &mut self,
        key: impl Into<ElementId>,
        cx: &mut App,
        init_query: impl FnOnce(
            &mut Self,
            &mut App,
        ) -> QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>,
    ) -> DbEntity<Mode::Output>
    where
        Db: Database + 'static,
        Adptr: Adapter + 'static,
        FieldUnion: unions::IsUnion + Send + Sync + 'static,
        FieldPath: Send + Sync + 'static,
        Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync + 'static,
        Fields::Type: SubscribableRow,
        Mode: SelectStmtFetchMode<Fields::Type> + Send + Sync + 'static,
        Mode::Output: Clone + PartialEq + Send,
    {
        let state_entity: Entity<DbQueryState<Mode::Output>> =
            self.use_keyed_state(key, cx, |_window, cx| {
                let data_entity = cx.new(|_cx| None);
                DbQueryState {
                    data_entity,
                    cancel_flag: None,
                    current_descriptor: None,
                }
            });

        let query = init_query(self, cx);
        maybe_resubscribe(state_entity.clone(), query, cx);

        let data_entity = state_entity.read(cx).data_entity.clone();
        DbEntity {
            entity: data_entity,
        }
    }

    #[track_caller]
    fn use_db_query<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>(
        &mut self,
        cx: &mut App,
        init_query: impl FnOnce(
            &mut Self,
            &mut App,
        ) -> QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>,
    ) -> DbEntity<Mode::Output>
    where
        Db: Database + 'static,
        Adptr: Adapter + 'static,
        FieldUnion: unions::IsUnion + Send + Sync + 'static,
        FieldPath: Send + Sync + 'static,
        Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync + 'static,
        Fields::Type: SubscribableRow,
        Mode: SelectStmtFetchMode<Fields::Type> + Send + Sync + 'static,
        Mode::Output: Clone + PartialEq + Send,
    {
        let state_entity: Entity<DbQueryState<Mode::Output>> =
            self.use_state(cx, |_window, cx| {
                let data_entity = cx.new(|_cx| None);
                DbQueryState {
                    data_entity,
                    cancel_flag: None,
                    current_descriptor: None,
                }
            });

        let query = init_query(self, cx);
        maybe_resubscribe(state_entity.clone(), query, cx);

        let data_entity = state_entity.read(cx).data_entity.clone();
        DbEntity {
            entity: data_entity,
        }
    }
}

fn maybe_resubscribe<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>(
    state_entity: Entity<DbQueryState<Mode::Output>>,
    query: QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>,
    cx: &mut App,
) where
    Db: Database + 'static,
    Adptr: Adapter + 'static,
    FieldUnion: unions::IsUnion + Send + Sync + 'static,
    FieldPath: Send + Sync + 'static,
    Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync + 'static,
    Fields::Type: SubscribableRow,
    Mode: SelectStmtFetchMode<Fields::Type> + Send + Sync + 'static,
    Mode::Output: Clone + PartialEq + Send,
{
    let new_descriptor = query.descriptor();

    let needs_subscribe = {
        let state = state_entity.read(cx);
        state
            .current_descriptor
            .as_ref()
            .map_or(true, |current| current != &new_descriptor)
    };

    if !needs_subscribe {
        return;
    }

    // Cancel old subscription if any.
    state_entity.update(cx, |state, _cx| {
        if let Some(flag) = state.cancel_flag.take() {
            flag.store(true, Ordering::Relaxed);
        }
        state.current_descriptor = Some(new_descriptor);
    });

    // Spawn new subscription.
    let data_entity = state_entity.read(cx).data_entity.clone();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    state_entity.update(cx, |state, _cx| {
        state.cancel_flag = Some(cancel_flag.clone());
    });

    spawn_subscription(query, data_entity, cancel_flag, cx);
}

fn spawn_subscription<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>(
    query: QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>,
    data_entity: Entity<Option<Mode::Output>>,
    cancel_flag: Arc<AtomicBool>,
    cx: &mut App,
) where
    Db: Database + 'static,
    Adptr: Adapter + 'static,
    FieldUnion: unions::IsUnion + Send + Sync + 'static,
    FieldPath: Send + Sync + 'static,
    Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync + 'static,
    Fields::Type: SubscribableRow,
    Mode: SelectStmtFetchMode<Fields::Type> + Send + Sync + 'static,
    Mode::Output: Clone + PartialEq + Send,
{
    let weak_data = data_entity.downgrade();

    cx.spawn(async move |cx: &mut AsyncApp| {
        let sub = query.subscribe().await.unwrap();

        // Bridge crossbeam (blocking) to async channel.
        // The bridge thread checks cancel_flag to know when to stop.
        let (tx, rx) = async_channel::unbounded();
        let bridge_cancel = cancel_flag.clone();
        std::thread::spawn(move || {
            while let Ok(_meta) = sub.recv() {
                if bridge_cancel.load(Ordering::Relaxed) {
                    break;
                }
                let data = sub.data().clone();
                if tx.send_blocking(data).is_err() {
                    break;
                }
            }
        });

        while let Ok(data) = rx.recv().await {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }
            let result = weak_data.update(cx, |state, cx| {
                *state = Some(data);
                cx.notify();
            });
            if result.is_err() {
                break; // Entity was dropped.
            }
        }
    })
    .detach();
}
