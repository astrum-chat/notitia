mod bert_embedder;

use bert_embedder::BertEmbedder;
use notitia::prelude::*;
use notitia::{ConnectionOptions, Notitia};
use notitia_sqlite::SqliteAdapter;

#[derive(Debug)]
#[database]
struct MyDatabase {
    users: Table<User>,

    #[db(foreign_key(user_id, users.id, on_delete = Cascade))]
    posts: Table<Post>,
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
    #[db(embed)]
    contents: String,
    user_id: String,
}

async fn populate_db(db: &Notitia<MyDatabase, SqliteAdapter>) -> anyhow::Result<()> {
    // Clear existing data so re-runs don't hit unique constraint errors.
    db.mutate(MyDatabase::POSTS.delete()).execute().await?;
    db.mutate(MyDatabase::USERS.delete()).execute().await?;

    db.mutate(
        MyDatabase::USERS.insert(
            User::build()
                .id("1")
                .name("Alice")
                .age(30)
                .is_premium(true)
                .email("alice@example.com"),
        ),
    )
    .execute()
    .await?;

    let posts = [
        (
            "1",
            "Hello World",
            "This is my first post on the platform. Excited to be here!",
        ),
        (
            "2",
            "Rust Tips",
            "Always use pattern matching instead of if-else chains when working with enums.",
        ),
        (
            "3",
            "Morning Coffee",
            "There's nothing like a freshly brewed cup of coffee to start the day.",
        ),
        (
            "4",
            "Vector Databases",
            "Embedding search is transforming how we build retrieval systems.",
        ),
        (
            "5",
            "Book Review",
            "Just finished reading Project Hail Mary. Absolutely phenomenal sci-fi.",
        ),
    ];

    for (id, title, contents) in posts {
        let post = Post::build()
            .id(id)
            .title(title)
            .contents(contents)
            .user_id("1");

        // Embedding sync happens automatically via MutationHook.
        db.mutate(MyDatabase::POSTS.insert(post)).execute().await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Loading BERT model...");
    let embedder = BertEmbedder::new().await?;
    println!("Model loaded.");

    let db = MyDatabase::connect::<SqliteAdapter>(
        ConnectionOptions::new("sqlite:./examples/app.db")
            .embeddings_uri("./examples/app_embeddings")
            .embedder(embedder),
    )
    .await?;

    populate_db(&db).await?;

    let query = "coffee";
    let posts = db
        .query(
            MyDatabase::POSTS
                .select((Post::TITLE, Post::CONTENTS))
                .search(Post::CONTENTS, query)
                .fetch_many::<Vec<_>>(10),
        )
        .execute()
        .await?;

    println!("\nSearch results for '{query}':");
    for post in &posts {
        println!("  - {:#?}", post);
    }

    Ok(())
}
