mod components;
mod element_id_ext;
mod schema;

use gpui::{
    AnyElement, App, Application, AsyncApp, ElementId, WeakEntity, Window, WindowOptions, div,
    prelude::*, px, rgb,
};
use notitia::{Database, Notitia, SelectStmtBuildable, SelectStmtSelectable};
use notitia_gpui::WindowNotitiaExt;
use notitia_sqlite::SqliteAdapter;
use smallvec::{SmallVec, smallvec};

use components::{AddTodoModal, TodoComponent};
use schema::{Todo, TodosDatabase};

use crate::{element_id_ext::ElementIdExt, schema::UniqueId};

struct Main {
    db: Option<Notitia<TodosDatabase, SqliteAdapter>>,
    show_add_todo: bool,
}

impl Render for Main {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let main_handle = cx.weak_entity();
        let show_modal = self.show_add_todo;

        let todos = self.db.as_ref().map(|db| {
            window.use_keyed_db_query("todos", cx, |_cx, _window| {
                db.query(
                    TodosDatabase::TODOS
                        .select((Todo::ID, Todo::TITLE, Todo::CONTENT, Todo::COMPLETED))
                        .fetch_all::<Vec<_>>(),
                )
            })
        });

        div()
            .size_full()
            .bg(rgb(0x1a1a1a))
            .flex()
            .flex_col()
            .gap(px(7.))
            .p(px(14.))
            .children(render_todos(
                "todos",
                todos.as_ref().and_then(|q| q.read(cx)),
                main_handle.clone(),
            ))
            .child(
                div()
                    .id("add-todo-btn")
                    .flex()
                    .justify_center()
                    .px(px(12.))
                    .py(px(6.))
                    .bg(rgb(0x4488ff))
                    .text_color(rgb(0xffffff))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0x3377ee)))
                    .active(|s| s.bg(rgb(0x2266dd)))
                    .child("Add Todo")
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.show_add_todo = !this.show_add_todo;
                        cx.notify();
                    })),
            )
            .when(show_modal, |div| {
                let main_handle = main_handle.clone();

                div.child(AddTodoModal::new(
                    "add-todo-modal",
                    move |title, content, _window, cx| {
                        let Some(db) = main_handle
                            .upgrade()
                            .and_then(|main| main.read(cx).db.clone())
                        else {
                            return;
                        };

                        cx.spawn(async move |_cx: &mut AsyncApp| {
                            let _ = db
                                .mutate(
                                    TodosDatabase::TODOS.insert(
                                        Todo::build()
                                            .id(UniqueId::new())
                                            .title(title.as_str())
                                            .content(content.as_str())
                                            .completed(false),
                                    ),
                                )
                                .execute()
                                .await;
                        })
                        .detach();
                    },
                ))
            })
    }
}

fn render_todos(
    base_id: impl Into<ElementId>,
    todos: Option<&Vec<(UniqueId, String, String, bool)>>,
    main_handle: WeakEntity<Main>,
) -> SmallVec<[AnyElement; 2]> {
    let base_id = base_id.into();

    match todos {
        Some(todos) => todos
            .iter()
            .enumerate()
            .map(|(idx, (id, title, content, completed))| {
                let id = id.clone();
                let title = title.clone();
                let content = content.clone();
                let completed = *completed;

                let on_toggle_handle = main_handle.clone();
                let on_delete_handle = main_handle.clone();
                let on_delete_id = id.clone();

                let on_toggle = move |_window: &mut Window, cx: &mut App| {
                    let Some(db) = on_toggle_handle
                        .upgrade()
                        .and_then(|main| main.read(cx).db.clone())
                    else {
                        return;
                    };

                    let on_toggle_id = id.clone();

                    cx.spawn(async move |_cx: &mut AsyncApp| {
                        let _ = db
                            .mutate(
                                TodosDatabase::TODOS
                                    .update(Todo::build().completed(!completed))
                                    .filter(Todo::ID.eq(on_toggle_id)),
                            )
                            .execute()
                            .await;
                    })
                    .detach();
                };

                let on_delete = move |_window: &mut Window, cx: &mut App| {
                    let Some(db) = on_delete_handle
                        .upgrade()
                        .and_then(|main| main.read(cx).db.clone())
                    else {
                        return;
                    };

                    let id = on_delete_id.clone();

                    cx.spawn(async move |_cx: &mut AsyncApp| {
                        let _ = db
                            .mutate(TodosDatabase::TODOS.delete().filter(Todo::ID.eq(id)))
                            .execute()
                            .await;
                    })
                    .detach();
                };

                TodoComponent::new(
                    base_id.with_suffix(idx.to_string()),
                    title,
                    content,
                    completed,
                    on_toggle,
                    on_delete,
                )
                .into_any_element()
            })
            .collect(),
        None => smallvec!["No todos found.".into_any_element()],
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_primitives::input::init(cx);

        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|cx| {
                let main = Main {
                    db: None,
                    show_add_todo: false,
                };

                cx.spawn(async |main, cx| {
                    let db = TodosDatabase::connect::<SqliteAdapter>("sqlite:./examples/app.db")
                        .await
                        .unwrap();

                    let _ = main.update(cx, |main: &mut Main, cx| {
                        main.db = Some(db.clone());
                        cx.notify();
                    });
                })
                .detach();

                main
            })
        })
        .unwrap();
    });
}
