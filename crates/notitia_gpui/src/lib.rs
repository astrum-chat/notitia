use gpui::{App, AsyncApp, Context, ElementId, Entity};
use notitia::{
    Adapter, Database, FieldKindGroup, QueryExecutor, SelectStmtFetchMode, SubscribableRow,
};

pub struct DbEntity<T: 'static> {
    entity: Entity<Option<T>>,
}

impl<T: 'static> DbEntity<T> {
    pub fn read<'a>(&self, cx: &'a App) -> Option<&'a T> {
        self.entity.read(cx).as_ref()
    }
}

pub trait WindowNotitiaExt {
    fn use_keyed_db_query<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>(
        &mut self,
        key: impl Into<ElementId>,
        cx: &mut App,
        init_query: impl FnOnce(
            &mut Self,
            &mut Context<Option<Mode::Output>>,
        )
            -> QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>,
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
            &mut Context<Option<Mode::Output>>,
        )
            -> QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>,
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
            &mut Context<Option<Mode::Output>>,
        )
            -> QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>,
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
        let entity = self.use_keyed_state(key, cx, |window, cx| {
            let query = init_query(window, cx);
            spawn_subscription(query, cx);
            None
        });

        DbEntity { entity }
    }

    #[track_caller]
    fn use_db_query<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>(
        &mut self,
        cx: &mut App,
        init_query: impl FnOnce(
            &mut Self,
            &mut Context<Option<Mode::Output>>,
        )
            -> QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>,
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
        let entity = self.use_state(cx, |window, cx| {
            let query = init_query(window, cx);
            spawn_subscription(query, cx);
            None
        });

        DbEntity { entity }
    }
}

fn spawn_subscription<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>(
    query: QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>,
    cx: &mut Context<Option<Mode::Output>>,
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
    cx.spawn(
        async move |state: gpui::WeakEntity<Option<Mode::Output>>, cx: &mut AsyncApp| {
            let sub = query.subscribe().await.unwrap();

            // sub.recv() is a blocking call (crossbeam), so run it on a
            // dedicated thread and bridge back via an async channel.
            let (tx, rx) = async_channel::unbounded();
            std::thread::spawn(move || {
                while let Ok(_meta) = sub.recv() {
                    let data = sub.data().clone();
                    if tx.send_blocking(data).is_err() {
                        break;
                    }
                }
            });

            while let Ok(data) = rx.recv().await {
                let _ = state.update(cx, |state: &mut Option<Mode::Output>, cx| {
                    *state = Some(data);
                    cx.notify();
                });
            }
        },
    )
    .detach();
}
