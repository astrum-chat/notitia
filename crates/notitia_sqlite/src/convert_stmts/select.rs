use notitia_core::{
    Database, Datatype, FieldFilter, FieldFilterMetadata, FieldKindGroup, OrderDirection,
    SelectStmtBuilt, SelectStmtFetchMode,
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
    match filter {
        FieldFilter::In(m) => {
            let col = Expr::col((Alias::new(m.left.table_name), Alias::new(m.left.field_name)));
            let values: Vec<sea_query::Value> = m.right.iter().map(datatype_to_sea_value).collect();
            col.is_in(values)
        }
        _ => {
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
                FieldFilter::In(_) => unreachable!(),
            };

            let col = Expr::col((
                Alias::new(metadata.left.table_name),
                Alias::new(metadata.left.field_name),
            ));
            let value = datatype_to_sea_value(&metadata.right);

            build(col, value)
        }
    }
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

    let field_names = stmt.fields.field_names();
    for name in &field_names {
        query.column(Alias::new(*name));
    }

    // Only add ORDER BY fields to the SELECT list when the fetch mode
    // needs order keys (fetch_all / fetch_many).
    if stmt.mode.needs_order_keys() {
        for order in &stmt.order_by {
            if !field_names.contains(&order.field) {
                query.column(Alias::new(order.field));
            }
        }
    }

    for table in &stmt.tables {
        query.from(Alias::new(*table));
    }

    for filter in &stmt.filters {
        query.and_where(filter_to_expr(filter));
    }

    // When similarity search is active, use CASE-based ordering by PK rank.
    // This preserves the zvec similarity ranking in the SQL results.
    // For typical topk sizes (10-100), CASE is the fastest approach in SQLite
    // — no temp tables, no joins, just a few integer comparisons per row.
    #[cfg(feature = "embeddings")]
    if let Some(ref pk_order) = stmt.similarity_pk_order {
        if !pk_order.is_empty() {
            // Find the pk field from the In filter we injected
            let pk_col = stmt
                .filters
                .iter()
                .find_map(|f| {
                    if let FieldFilter::In(m) = f {
                        Some(m.left.field_name)
                    } else {
                        None
                    }
                })
                .unwrap_or("");

            let mut case = sea_query::CaseStatement::new();
            for (i, pk) in pk_order.iter().enumerate() {
                case = case.case(
                    Expr::col(Alias::new(pk_col)).eq(pk.as_str()),
                    Expr::val(i as i32),
                );
            }
            query.order_by_expr(case.into(), sea_query::Order::Asc);
        }
    }

    for order in &stmt.order_by {
        let col = Expr::col((Alias::new(order.table), Alias::new(order.field)));
        match order.direction {
            OrderDirection::Asc => {
                query.order_by_expr(col.into(), sea_query::Order::Asc);
            }
            OrderDirection::Desc => {
                query.order_by_expr(col.into(), sea_query::Order::Desc);
            }
        }
    }

    query.to_string(SqliteQueryBuilder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notitia_core::{
        OrderDirection, SelectStmtBuildable, SelectStmtFilterable, SelectStmtOrderable,
        SelectStmtSelectable, Table,
    };
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

    #[test]
    fn select_with_order_by_asc() {
        let stmt = TestDb::USERS
            .select(User::NAME)
            .order_by(User::AGE, OrderDirection::Asc)
            .fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(
            sql,
            r#"SELECT "name" FROM "users" ORDER BY "users"."age" ASC"#
        );
    }

    #[test]
    fn select_with_order_by_desc() {
        let stmt = TestDb::USERS
            .select(User::NAME)
            .order_by(User::NAME, OrderDirection::Desc)
            .fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(
            sql,
            r#"SELECT "name" FROM "users" ORDER BY "users"."name" DESC"#
        );
    }

    #[test]
    fn select_with_multiple_order_by() {
        let stmt = TestDb::USERS
            .select(User::NAME)
            .order_by(User::AGE, OrderDirection::Desc)
            .order_by(User::NAME, OrderDirection::Asc)
            .fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(
            sql,
            r#"SELECT "name" FROM "users" ORDER BY "users"."age" DESC, "users"."name" ASC"#
        );
    }

    #[test]
    fn select_with_filter_and_order_by() {
        let stmt = TestDb::USERS
            .select(User::NAME)
            .filter(User::AGE.gte(18i64))
            .order_by(User::NAME, OrderDirection::Asc)
            .fetch_one();
        let sql = select_stmt_to_sql(&stmt);

        assert_eq!(
            sql,
            r#"SELECT "name" FROM "users" WHERE "users"."age" >= 18 ORDER BY "users"."name" ASC"#
        );
    }
}
