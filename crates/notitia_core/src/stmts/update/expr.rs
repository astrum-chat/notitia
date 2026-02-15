use crate::Datatype;

/// A composable expression tree for update field values.
///
/// Allows both literal values and field-reference-based expressions
/// (e.g. `SET content = content || 'chunk'`).
#[derive(Clone, Debug)]
pub enum FieldExpr {
    /// A literal value: `SET field = 'value'`
    Literal(Datatype),
    /// A reference to a field's current value: `SET field = other_field`
    Field(&'static str),
    /// String concatenation: `SET field = left || right`
    Concat(Box<FieldExpr>, Box<FieldExpr>),
}

impl FieldExpr {
    /// Resolve this expression against the current row values.
    /// Used by subscription merge to compute the new value locally.
    pub fn resolve(&self, row: &[(&'static str, Datatype)]) -> Datatype {
        match self {
            FieldExpr::Literal(val) => val.clone(),
            FieldExpr::Field(name) => row
                .iter()
                .find_map(|(k, v)| if *k == *name { Some(v.clone()) } else { None })
                .unwrap_or(Datatype::Null),
            FieldExpr::Concat(left, right) => {
                let l = left.resolve(row);
                let r = right.resolve(row);
                match (l, r) {
                    (Datatype::Text(a), Datatype::Text(b)) => {
                        let mut result = a;
                        result.push_str(&b);
                        Datatype::Text(result)
                    }
                    (_, r) => r,
                }
            }
        }
    }
}

// Raw values that convert to Datatype automatically become Literal.
impl<T: Into<Datatype>> From<T> for FieldExpr {
    fn from(val: T) -> Self {
        FieldExpr::Literal(val.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_resolve() {
        let expr = FieldExpr::Literal(Datatype::Text("hello".into()));
        let row = vec![];
        assert_eq!(expr.resolve(&row), Datatype::Text("hello".into()));
    }

    #[test]
    fn field_resolve() {
        let expr = FieldExpr::Field("name");
        let row = vec![("name", Datatype::Text("Alice".into()))];
        assert_eq!(expr.resolve(&row), Datatype::Text("Alice".into()));
    }

    #[test]
    fn field_resolve_missing() {
        let expr = FieldExpr::Field("missing");
        let row = vec![("name", Datatype::Text("Alice".into()))];
        assert_eq!(expr.resolve(&row), Datatype::Null);
    }

    #[test]
    fn concat_field_literal() {
        let expr = FieldExpr::Concat(
            Box::new(FieldExpr::Field("content")),
            Box::new(FieldExpr::Literal(Datatype::Text(" chunk".into()))),
        );
        let row = vec![("content", Datatype::Text("hello".into()))];
        assert_eq!(expr.resolve(&row), Datatype::Text("hello chunk".into()));
    }

    #[test]
    fn concat_two_fields() {
        let expr = FieldExpr::Concat(
            Box::new(FieldExpr::Field("first")),
            Box::new(FieldExpr::Field("last")),
        );
        let row = vec![
            ("first", Datatype::Text("John".into())),
            ("last", Datatype::Text("Doe".into())),
        ];
        assert_eq!(expr.resolve(&row), Datatype::Text("JohnDoe".into()));
    }

    #[test]
    fn nested_concat() {
        let expr = FieldExpr::Concat(
            Box::new(FieldExpr::Concat(
                Box::new(FieldExpr::Field("a")),
                Box::new(FieldExpr::Literal(Datatype::Text("b".into()))),
            )),
            Box::new(FieldExpr::Literal(Datatype::Text("c".into()))),
        );
        let row = vec![("a", Datatype::Text("a".into()))];
        assert_eq!(expr.resolve(&row), Datatype::Text("abc".into()));
    }

    #[test]
    fn from_string() {
        let expr: FieldExpr = "hello".to_string().into();
        match expr {
            FieldExpr::Literal(Datatype::Text(s)) => assert_eq!(s, "hello"),
            _ => panic!("Expected Literal(Text)"),
        }
    }

    #[test]
    fn from_i64() {
        let expr: FieldExpr = 42i64.into();
        match expr {
            FieldExpr::Literal(Datatype::BigInt(v)) => assert_eq!(v, 42),
            _ => panic!("Expected Literal(BigInt)"),
        }
    }
}
