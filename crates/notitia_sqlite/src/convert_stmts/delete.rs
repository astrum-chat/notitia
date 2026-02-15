use notitia_core::FieldFilter;
use sea_query::{Alias, Query, SqliteQueryBuilder};

use super::select::filter_to_expr;

pub fn delete_stmt_to_sql(table_name: &str, filters: &[FieldFilter]) -> String {
    let mut query = Query::delete();

    query.from_table(Alias::new(table_name));

    for filter in filters {
        query.and_where(filter_to_expr(filter));
    }

    query.to_string(SqliteQueryBuilder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notitia_core::Table;
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
    fn delete_all() {
        let stmt = TestDb::USERS.delete();
        let sql = delete_stmt_to_sql(stmt.table_name, &[]);

        assert_eq!(sql, r#"DELETE FROM "users""#);
    }

    #[test]
    fn delete_with_filter() {
        let stmt = TestDb::USERS.delete().filter(User::ID.eq("abc"));
        let sql = delete_stmt_to_sql(stmt.table_name, &stmt.filters);

        assert_eq!(sql, r#"DELETE FROM "users" WHERE "users"."id" = 'abc'"#);
    }

    #[test]
    fn delete_with_multiple_filters() {
        let stmt = TestDb::USERS
            .delete()
            .filter(User::ID.eq("abc"))
            .filter(User::AGE.gt(18i64));
        let sql = delete_stmt_to_sql(stmt.table_name, &stmt.filters);

        assert_eq!(
            sql,
            r#"DELETE FROM "users" WHERE "users"."id" = 'abc' AND "users"."age" > 18"#
        );
    }
}
