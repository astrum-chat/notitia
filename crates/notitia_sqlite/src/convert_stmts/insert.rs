use notitia_core::Datatype;
use sea_query::{Alias, Expr, Query, SqliteQueryBuilder};

use super::select::datatype_to_sea_value;

pub fn insert_stmt_to_sql(table_name: &str, fields: &[(&str, Datatype)]) -> String {
    let mut query = Query::insert();

    query.into_table(Alias::new(table_name));

    let columns: Vec<_> = fields.iter().map(|(name, _)| Alias::new(*name)).collect();
    query.columns(columns);

    let values: Vec<_> = fields
        .iter()
        .map(|(_, datatype)| Expr::val(datatype_to_sea_value(datatype)).into())
        .collect();
    query.values_panic(values);

    query.to_string(SqliteQueryBuilder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notitia_core::{Record, Table};
    use notitia_macros::{database, record};

    #[derive(Debug)]
    #[database]
    struct TestDb {
        users: Table<User>,
    }

    #[derive(Debug)]
    #[record]
    struct User {
        #[db(primary_key)]
        id: String,
        name: String,
        age: i64,
    }

    #[test]
    fn insert_single_record() {
        let user = User::build().id("abc").name("Bob").age(36);
        let stmt = TestDb::USERS.insert(user);

        let fields = stmt.record.into_datatypes();
        let sql = insert_stmt_to_sql(stmt.table_name, &fields);

        assert_eq!(
            sql,
            r#"INSERT INTO "users" ("id", "name", "age") VALUES ('abc', 'Bob', 36)"#
        );
    }
}
