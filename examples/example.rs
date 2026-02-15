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
        .query(MyDatabase::USERS.select(User::NAME).fetch_all::<Vec<_>>())
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

    db.mutate(
        MyDatabase::USERS
            .update(User::build().name("Alice"))
            .filter(User::ID.eq("0")),
    )
    .execute()
    .await?;

    // Mutate a couple times to trigger subscription events
    db.mutate(
        MyDatabase::USERS
            .update(User::build().name("Jeff"))
            .filter(User::ID.eq("0")),
    )
    .execute()
    .await?;

    // Give the background task time to print
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Ok(())
}

/*
trait DefaultDatatype {
    fn default() -> Self;
*/

//MyDatabase::USERS.insert(User::Row {});

/*let get_user_info = MyDatabase::USERS
.join(MyDatabase::POSTS)
.join(MyDatabase::CABBAGE)
.select((User::ID, User::EMAIL, Cabbage::ID))
.filter(User::ID.eq("12345"))
.fetch_one();*/

//let x = get_user_info.execute(&db).await;

//Ok(())

/*let db = MyDatabase::open::<Sqlite>("db.sqlite");

// Gets one user, returns an error if there is more than one row.
//let one_user = db.users.get_one((User::NAME, User::AGE)).execute();

// Gets one user.
//let first_user = db.users.get_first((User::NAME, User::AGE)).execute();

// Gets all users.
//let all_users: Vec<(String, i64)> = db.users.get_all((User::NAME, User::AGE)).execute();

// Gets a maximum of n users.
/*let n_users: SmallVec<[(String, i64); 5]> = db
.users
.get_n::<10, _, _>((User::NAME, User::AGE))
.execute();*/

let get_user_info = MyDatabase::USERS
    .join(MyDatabase::POSTS)
    .join(MyDatabase::CABBAGE)
    .select((User::ID, User::EMAIL, Cabbage::ID))
    .filter(User::ID.eq("12345"))
    .fetch_one();

let x = get_user_info.execute(&db).await;

println!("{:#?}", get_user_info);

//let exec = get_user_info.execute();

//let (name, is_premium) = get_user_info.execute(&db).await;

/*db.prepare::<String>(|id| {
    db.users
        .select(User::NAME)
        .filter(User::ID.eq(id))
        .fetch_one()
});*/

struct PreparedArg<T> {
    _ty: PhantomData<T>,
}

//let user_info = window.use_query(cx, get_user_info);

//let (user_name, user_id) = user_info.read(cx);

//println!("{:#?}", query);

// Selects the ID field from users.
//let query_1 = db.users.join(&db.posts).select(User::AGE);
//.filter(User::AGE.eq(5));
//.filter(Post::ID.eq("hello"));
//

//println!("{:#?}", query_1);

// Selects the ID fields from users and posts.
//let query_2 = db.users.join(&db.posts);

// Can't select the ID from posts as its not in scope.
//let query_3 = db.users.select(Post::ID);

//let x = join_2(Post::ID);
//let x = join(Post::ID);
//let x = join(Cabbage::ID);

//.filter(Post::ID.eq("12345"))
//.join(db.users, Post::ID);
//.get_first(Post::TITLE)
//.filter(Post::ID.eq("12345"));*/
//}

#[database]
struct CoolDb {
    users: Table<User>,
}
