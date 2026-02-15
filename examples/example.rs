use notitia::prelude::*;
use notitia_sqlite::SqliteAdapter;

#[derive(Debug)]
#[database]
struct MyDatabase {
    users: Table<User>,

    #[db(foreign_key(user_id, users.id, on_delete = Cascade))]
    posts: Table<Post>,
    cabbage: Table<Cabbage>,
}

#[derive(Debug)]
#[record]
struct User {
    #[db(primary_key)]
    id: String,
    name: String,
    age: i64,
    is_premium: bool,
    #[db(unique)]
    email: String,
}

#[derive(Debug)]
#[record]
struct Post {
    #[db(primary_key)]
    id: String,
    title: String,
    contents: Option<String>,
    user_id: String,
}

#[derive(Debug)]
#[record]
struct Cabbage {
    #[db(primary_key)]
    id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = MyDatabase::connect::<SqliteAdapter>("sqlite:./examples/app.db").await?;

    let subscription = db
        .query(
            MyDatabase::USERS
                .select((User::NAME, User::AGE))
                .order_by(User::AGE, OrderDirection::Asc)
                .fetch_all::<BTreeMap<_, _>>(),
        )
        .subscribe()
        .await?;

    // Poll subscription in a background task
    tokio::spawn(async move {
        while let Ok(meta) = subscription.recv() {
            let data = subscription.data();
            println!("Subscription event:");
            println!("  Data: {data:#?}");
            println!("  Metadata: {meta:#?}");
            println!();
        }
    });

    /*db.mutate(
        MyDatabase::USERS
            .update(User::build().name("Alice"))
            .filter(User::ID.eq("0")),
    )
    .execute()
    .await?;*/

    /*db.mutate(
        MyDatabase::USERS.insert(
            User::build()
                .id("1")
                .name("Bob")
                .age(64)
                .is_premium(false)
                .email("example@example.example"),
        ),
    )
    .execute()
    .await;*/

    // Mutate a couple times to trigger subscription events
    /*db.mutate(
        MyDatabase::USERS
            .update(User::build().name("Jeff"))
            .filter(User::ID.eq("0")),
    )
    .execute()
    .await?;*/

    // Give the background task time to print
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Ok(())
}
