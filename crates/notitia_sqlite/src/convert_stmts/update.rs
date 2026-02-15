use notitia_core::{FieldExpr, FieldFilter};
use sea_query::{Alias, Expr, Query, SimpleExpr, SqliteQueryBuilder};

use super::select::{datatype_to_sea_value, filter_to_expr};

fn field_expr_to_sea_expr(expr: &FieldExpr) -> SimpleExpr {
    match expr {
        FieldExpr::Literal(val) => Expr::val(datatype_to_sea_value(val)).into(),
        FieldExpr::Field(name) => Expr::col(Alias::new(*name)).into(),
        FieldExpr::Concat(left, right) => {
            let l = field_expr_to_sea_expr(left);
            let r = field_expr_to_sea_expr(right);
            SimpleExpr::Binary(
                Box::new(l),
                sea_query::BinOper::Custom("||"),
                Box::new(r),
            )
        }
    }
}

pub fn update_stmt_to_sql(
    table_name: &str,
    fields: &[(&str, FieldExpr)],
    filters: &[FieldFilter],
) -> String {
    let mut query = Query::update();

    query.table(Alias::new(table_name));

    for (name, expr) in fields {
        query.value(Alias::new(*name), field_expr_to_sea_expr(expr));
    }

    for filter in filters {
        query.and_where(filter_to_expr(filter));
    }

    query.to_string(SqliteQueryBuilder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notitia_core::{PartialRecord, Table};
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
    fn update_all_fields() {
        let user = User::build().id("abc").name("Bob").age(36i64);
        let stmt = TestDb::USERS.update(user);

        let fields = stmt.partial.into_set_fields();
        let sql = update_stmt_to_sql(stmt.table_name, &fields, &[]);

        assert_eq!(
            sql,
            r#"UPDATE "users" SET "id" = 'abc', "name" = 'Bob', "age" = 36"#
        );
    }

    #[test]
    fn update_partial_fields() {
        let partial = User::build().name("Alice");
        let stmt = TestDb::USERS.update(partial).filter(User::ID.eq("abc"));

        let fields = stmt.partial.into_set_fields();
        let sql = update_stmt_to_sql(stmt.table_name, &fields, &stmt.filters);

        assert_eq!(
            sql,
            r#"UPDATE "users" SET "name" = 'Alice' WHERE "users"."id" = 'abc'"#
        );
    }

    #[test]
    fn update_with_filter() {
        let user = User::build().id("abc").name("Bob").age(36i64);
        let stmt = TestDb::USERS.update(user).filter(User::ID.eq("abc"));

        let fields = stmt.partial.into_set_fields();
        let sql = update_stmt_to_sql(stmt.table_name, &fields, &stmt.filters);

        assert_eq!(
            sql,
            r#"UPDATE "users" SET "id" = 'abc', "name" = 'Bob', "age" = 36 WHERE "users"."id" = 'abc'"#
        );
    }

    #[test]
    fn update_with_multiple_filters() {
        let user = User::build().id("abc").name("Bob").age(36i64);
        let stmt = TestDb::USERS
            .update(user)
            .filter(User::ID.eq("abc"))
            .filter(User::AGE.gt(18i64));

        let fields = stmt.partial.into_set_fields();
        let sql = update_stmt_to_sql(stmt.table_name, &fields, &stmt.filters);

        assert_eq!(
            sql,
            r#"UPDATE "users" SET "id" = 'abc', "name" = 'Bob', "age" = 36 WHERE "users"."id" = 'abc' AND "users"."age" > 18"#
        );
    }

    #[test]
    fn update_with_concat_expression() {
        let partial = User::build().name(User::NAME.concat(" Jr."));
        let stmt = TestDb::USERS.update(partial).filter(User::ID.eq("abc"));

        let fields = stmt.partial.into_set_fields();
        let sql = update_stmt_to_sql(stmt.table_name, &fields, &stmt.filters);

        assert_eq!(
            sql,
            r#"UPDATE "users" SET "name" = "name" || ' Jr.' WHERE "users"."id" = 'abc'"#
        );
    }

    #[test]
    fn update_with_field_reference() {
        let partial = User::build().name(User::ID);
        let stmt = TestDb::USERS.update(partial).filter(User::ID.eq("abc"));

        let fields = stmt.partial.into_set_fields();
        let sql = update_stmt_to_sql(stmt.table_name, &fields, &stmt.filters);

        assert_eq!(
            sql,
            r#"UPDATE "users" SET "name" = "id" WHERE "users"."id" = 'abc'"#
        );
    }
}
