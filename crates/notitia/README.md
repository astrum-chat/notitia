# Notitia

A type-safe, reactive Rust ORM with compile-time checked queries and built-in live subscriptions.

> In very early development, currently only supports basic queries and mutations.

Notitia separates database operations into two categories:

- **Queries** (`db.query(...)`) — read data. Queries can be executed once with `.execute()`, or turned into live subscriptions with `.subscribe()` that automatically stay in sync as data changes.

- **Mutations** (`db.mutate(...)`) — write data via insert, update, and delete operations. When a mutation executes, it broadcasts an event to relevant active subscriptions so they can merge the change into their local data without re-querying.

## Quick Start

### Define Your Schema

```rust
use notitia::prelude::*;

#[database]
struct MyDb {
    users: Table<User>,
}

#[record]
struct User {
    #[db(primary_key)]
    id: String,
    name: String,
    age: i64,
}
```

### Connect

```rust
use notitia_sqlite::SqliteAdapter;

let db = MyDb::connect::<SqliteAdapter>("sqlite:./app.db").await?;
```

### Query

```rust
use notitia::prelude::*;

// Fetch all users
let users = db
    .query(
        MyDb::USERS
            .select((User::ID, User::NAME, User::AGE))
            .fetch_all::<Vec<_>>(),
    )
    .execute()
    .await?;

// Fetch with filters
let adults = db
    .query(
        MyDb::USERS
            .select((User::ID, User::NAME))
            .filter(User::AGE.gte(18i64))
            .fetch_all::<Vec<_>>(),
    )
    .execute()
    .await?;

// Fetch a single row
let user = db
    .query(
        MyDb::USERS
            .select((User::ID, User::NAME))
            .filter(User::ID.eq("abc"))
            .fetch_one(),
    )
    .execute()
    .await?;
```

### Insert

```rust
db.mutate(
    MyDb::USERS.insert(
        User::build().id("abc").name("Alice").age(30),
    ),
)
.execute()
.await?;
```

### Update

```rust
// Update specific fields with a filter
db.mutate(
    MyDb::USERS
        .update(User::build().name("Bob"))
        .filter(User::ID.eq("abc")),
)
.execute()
.await?;
```

### Delete

```rust
db.mutate(
    MyDb::USERS
        .delete()
        .filter(User::ID.eq("abc")),
)
.execute()
.await?;
```

### Subscribe to Changes

Subscriptions receive live updates when mutations occur on matching rows.

```rust
let subscription = db
    .query(
        MyDb::USERS
            .select((User::ID, User::NAME, User::AGE))
            .fetch_all::<Vec<_>>(),
    )
    .subscribe()
    .await?;

// Initial data
let data = subscription.data();

// Block until a change arrives
let event = subscription.recv()?;
let updated_data = subscription.data();
```

<details>
<summary><h2>Migrations</h2></summary>

Notitia tracks schema changes through YAML snapshots and checks compatibility between versions to catch breaking changes before they ship.

### Setup

```sh
notitia init
```

This creates a `snapshots/` directory and a `notitia.toml` config file in your project.

### Taking Snapshots

```sh
notitia snapshot
```

Extracts the current schema from your database types and saves a versioned YAML snapshot. The command checks compatibility against all existing snapshots first — if any breaking changes are detected, the snapshot is rejected.

### Checking Compatibility

```sh
notitia check
```

Validates your current schema against all existing snapshots and reports any incompatibilities with actionable hints.

### Renaming Tables & Fields

Use `#[db(migrate_from(...))]` to tell the migration system that a table or field was renamed rather than removed:

```rust
#[database]
struct MyDb {
    #[db(migrate_from(legacy_users))]
    users: Table<User>,
}

#[record]
struct User {
    #[db(migrate_from(user_id))]
    id: String,
    name: String,
}
```

### Removing Tables & Fields

Declare intentional removals so the compatibility checker doesn't flag them as errors:

```rust
#[database(removed_tables(old_sessions))]
struct MyDb {
    users: Table<User>,
}

#[record(removed_fields(legacy_field))]
struct User {
    #[db(primary_key)]
    id: String,
    name: String,
}
```

### CLI Flags

| Flag | Effect |
|---|---|
| `-v, --verbose` | Show full cargo output during schema extraction |
| `--tmp` | Use a temporary directory instead of `.notitia/` |
| `-c, --crate <NAME>` | Target a specific workspace member by name |

</details>

## Reference

<details>
<summary>Fetch Modes</summary>

| Method | Returns |
|---|---|
| `.fetch_one()` | Exactly one row (errors if 0 or >1) |
| `.fetch_first()` | The first row (errors if 0) |
| `.fetch_all::<Vec<_>>()` | All matching rows |
| `.fetch_many::<Vec<_>>(n)` | Up to `n` rows |

</details>

<details>
<summary>Filter Operators</summary>

| Method | Meaning |
|---|---|
| `.eq(val)` | Equal to |
| `.ne(val)` | Not equal to |
| `.gt(val)` | Greater than |
| `.lt(val)` | Less than |
| `.gte(val)` | Greater than or equal to |
| `.lte(val)` | Less than or equal to |

</details>

<details>
<summary>Record Attributes</summary>

| Attribute | Effect |
|---|---|
| `#[db(primary_key)]` | Marks the field as a primary key |
| `#[db(unique)]` | Adds a unique constraint |

</details>

<details>
<summary>Custom Types</summary>

To use a custom type in a record, implement `AsDatatypeKind`, `Into<Datatype>`, and `TryFrom<Datatype>`:

```rust
use notitia::{AsDatatypeKind, Datatype, DatatypeKind, DatatypeKindMetadata, DatatypeConversionError};

#[derive(Clone, PartialEq)]
struct MyId(String);

impl AsDatatypeKind for MyId {
    fn as_datatype_kind() -> DatatypeKind {
        DatatypeKind::Text(DatatypeKindMetadata::default())
    }
}

impl Into<Datatype> for MyId {
    fn into(self) -> Datatype {
        Datatype::Text(self.0)
    }
}

impl TryFrom<Datatype> for MyId {
    type Error = DatatypeConversionError;

    fn try_from(d: Datatype) -> Result<Self, Self::Error> {
        String::try_from(d).map(MyId)
    }
}
```

</details>
