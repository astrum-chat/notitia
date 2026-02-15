use notitia_core::{
    Database, Datatype, FieldFilter, FieldFilterMetadata, FieldKindGroup, SelectStmtBuilt,
    SelectStmtFetchMode,
};
use sea_query::{Alias, Expr, Query, SimpleExpr, SqliteQueryBuilder};
use unions::IsUnion;

pub(crate) fn datatype_to_sea_value(datatype: &Datatype) -> sea_query::Value {
    match datatype {
        Datatype::Int(v) => sea_query::Value::Int(Some(*v)),
        Datatype::BigInt(v) => sea_query::Value::BigInt(Some(*v)),
        Datatype::Float(v) => sea_query::Value::Float(Some(*v)),
        Datatype::Double(v) => sea_query::Value::Double(Some(*v)),
        Datatype::Text(v) => sea_query::Value::String(Some(Box::new(v.clone()))),
        Datatype::Blob(v) => sea_query::Value::Bytes(Some(Box::new(v.clone()))),
        Datatype::Bool(v) => sea_query::Value::Bool(Some(*v)),
        Datatype::Null => sea_query::Value::Int(None),
    }
}

pub(crate) fn filter_to_expr(filter: &FieldFilter) -> SimpleExpr {
    let (metadata, build): (
        &FieldFilterMetadata,
        fn(Expr, sea_query::Value) -> SimpleExpr,
    ) = match filter {
        FieldFilter::Eq(m) => (m, |col, val| col.eq(val)),
        FieldFilter::Gt(m) => (m, |col, val| col.gt(val)),
        FieldFilter::Lt(m) => (m, |col, val| col.lt(val)),
        FieldFilter::Gte(m) => (m, |col, val| col.gte(val)),
        FieldFilter::Lte(m) => (m, |col, val| col.lte(val)),
        FieldFilter::Ne(m) => (m, |col, val| col.ne(val)),
    };

    let col = Expr::col((
        Alias::new(metadata.left.table_name),
        Alias::new(metadata.left.field_name),
    ));
    let value = datatype_to_sea_value(&metadata.right);

    build(col, value)
}

pub fn select_stmt_to_sql<Db, FieldUnion, FieldPath, Fields, Mode>(
    stmt: &SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Mode>,
) -> String
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
    Mode: SelectStmtFetchMode<Fields::Type>,
{
    let mut query = Query::select();

    for name in stmt.fields.field_names() {
        query.column(Alias::new(name));
    }

    for table in &stmt.tables {
        query.from(Alias::new(*table));
    }

    for filter in &stmt.filters {
        query.and_where(filter_to_expr(filter));
    }

    query.to_string(SqliteQueryBuilder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notitia_core::{SelectStmtBuildable, SelectStmtFilterable, SelectStmtSelectable, Table};
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
    fn select_all_no_filters() {
        let stmt = TestDb::USERS.select(User::NAME).fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(sql, r#"SELECT "name" FROM "users""#);
    }

    #[test]
    fn select_with_eq_filter() {
        let stmt = TestDb::USERS
            .select(User::NAME)
            .filter(User::ID.eq("abc"))
            .fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(
            sql,
            r#"SELECT "name" FROM "users" WHERE "users"."id" = 'abc'"#
        );
    }

    #[test]
    fn select_with_gt_filter() {
        let stmt = TestDb::USERS
            .select(User::NAME)
            .filter(User::AGE.gt(18i64))
            .fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(
            sql,
            r#"SELECT "name" FROM "users" WHERE "users"."age" > 18"#
        );
    }

    #[test]
    fn select_with_lt_filter() {
        let stmt = TestDb::USERS
            .select(User::AGE)
            .filter(User::AGE.lt(30i64))
            .fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(sql, r#"SELECT "age" FROM "users" WHERE "users"."age" < 30"#);
    }

    #[test]
    fn select_with_gte_filter() {
        let stmt = TestDb::USERS
            .select(User::AGE)
            .filter(User::AGE.gte(21i64))
            .fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(
            sql,
            r#"SELECT "age" FROM "users" WHERE "users"."age" >= 21"#
        );
    }

    #[test]
    fn select_with_lte_filter() {
        let stmt = TestDb::USERS
            .select(User::AGE)
            .filter(User::AGE.lte(65i64))
            .fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(
            sql,
            r#"SELECT "age" FROM "users" WHERE "users"."age" <= 65"#
        );
    }

    #[test]
    fn select_with_ne_filter() {
        let stmt = TestDb::USERS
            .select(User::NAME)
            .filter(User::NAME.ne("admin"))
            .fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(
            sql,
            r#"SELECT "name" FROM "users" WHERE "users"."name" <> 'admin'"#
        );
    }

    #[test]
    fn select_with_multiple_filters() {
        let stmt = TestDb::USERS
            .select(User::NAME)
            .filter(User::AGE.gte(18i64))
            .filter(User::AGE.lt(65i64))
            .fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(
            sql,
            r#"SELECT "name" FROM "users" WHERE "users"."age" >= 18 AND "users"."age" < 65"#
        );
    }
}
