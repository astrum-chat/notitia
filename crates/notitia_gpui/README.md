# Notitia GPUI

Reactive database queries for [GPUI](https://github.com/zed-industries/zed) applications, powered by [Notitia](../notitia/README.md).

`notitia_gpui` bridges Notitia's subscription system with GPUI's reactive rendering model. Database queries automatically re-render your UI when the underlying data changes.

## Quick Start

### Setup

Store a `Notitia` instance in your component and connect on startup:

```rust
use notitia::{Database, Notitia};
use notitia_sqlite::SqliteAdapter;

struct MyApp {
    db: Option<Notitia<MyDb, SqliteAdapter>>,
}

// Connect asynchronously during initialization
cx.spawn(async |app, cx| {
    let db = MyDb::connect::<SqliteAdapter>("sqlite:./app.db").await.unwrap();

    let _ = app.update(cx, |app: &mut MyApp, cx| {
        app.db = Some(db);
        cx.notify();
    });
})
.detach();
```

### Reactive Queries

Import the `WindowNotitiaExt` trait and use `use_keyed_db_query` inside your `Render` implementation:

```rust
use notitia::{SelectStmtBuildable, SelectStmtSelectable};
use notitia_gpui::WindowNotitiaExt;

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let users = self.db.as_ref().map(|db| {
            window.use_keyed_db_query("users", cx, |_window, _cx| {
                db.query(
                    MyDb::USERS
                        .select((User::ID, User::NAME))
                        .fetch_all::<Vec<_>>(),
                )
            })
        });

        div()
            .children(
                users
                    .as_ref()
                    .and_then(|q| q.read(cx))
                    .map(|rows| {
                        rows.iter()
                            .map(|(id, name)| div().child(name.clone()).into_any_element())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            )
    }
}
```

## Example

See the [todos example](./examples/todos/) for a complete working app with reactive queries, inserts, updates, and deletes.

Run it with:

```sh
cargo run -p notitia_gpui --example todos
```
