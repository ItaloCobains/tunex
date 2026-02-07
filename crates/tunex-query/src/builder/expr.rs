use crate::Value;

#[derive(Debug, Clone)]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Like,
    IsNull,
    IsNotNull,
    In,
}

#[derive(Debug, Clone)]
pub struct Condition {
    pub column: String,
    pub op: Op,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone)]
pub enum WhereClause {
    Condition(Condition),
    And(Vec<WhereClause>),
    Or(Vec<WhereClause>),
}

impl WhereClause {
    pub fn and(self, other: WhereClause) -> WhereClause {
        match self {
            WhereClause::And(mut clauses) => {
                clauses.push(other);
                WhereClause::And(clauses)
            }
            _ => WhereClause::And(vec![self, other]),
        }
    }

    pub fn or(self, other: WhereClause) -> WhereClause {
        match self {
            WhereClause::Or(mut clauses) => {
                clauses.push(other);
                WhereClause::Or(clauses)
            }
            _ => WhereClause::Or(vec![self, other]),
        }
    }

    pub fn to_sql(&self, idx: &mut i32) -> (String, Vec<Value>) {
        match self {
            WhereClause::Condition(c) => condition_to_sql(c, idx),
            WhereClause::And(clauses) => group_to_sql(clauses, "AND", idx),
            WhereClause::Or(clauses) => group_to_sql(clauses, "OR", idx),
        }
    }
}

fn condition_to_sql(c: &Condition, idx: &mut i32) -> (String, Vec<Value>) {
    match c.op {
        Op::IsNull => (format!("{} IS NULL", c.column), vec![]),
        Op::IsNotNull => (format!("{} IS NOT NULL", c.column), vec![]),
        Op::In => {
            let placeholders: Vec<String> = c
                .values
                .iter()
                .map(|_| {
                    *idx += 1;
                    format!("${}", idx)
                })
                .collect();
            let sql = format!("{} IN ({})", c.column, placeholders.join(", "));
            (sql, c.values.clone())
        }
        ref op => {
            *idx += 1;
            let symbol = match op {
                Op::Eq => "=",
                Op::Ne => "!=",
                Op::Lt => "<",
                Op::Gt => ">",
                Op::Le => "<=",
                Op::Ge => ">=",
                Op::Like => "LIKE",
                _ => unreachable!(),
            };
            let sql = format!("{} {} ${}", c.column, symbol, idx);
            (sql, c.values.clone())
        }
    }
}

fn group_to_sql(clauses: &[WhereClause], joiner: &str, idx: &mut i32) -> (String, Vec<Value>) {
    let mut parts = Vec::new();
    let mut values = Vec::new();
    for clause in clauses {
        let (sql, vals) = clause.to_sql(idx);
        parts.push(sql);
        values.extend(vals);
    }
    let sql = format!("({})", parts.join(&format!(" {} ", joiner)));
    (sql, values)
}

pub struct Expr;

impl Expr {
    pub fn col(name: &str) -> ExprColumn {
        ExprColumn {
            name: name.to_string(),
        }
    }
}

pub struct ExprColumn {
    name: String,
}

impl ExprColumn {
    pub fn eq(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition(Condition {
            column: self.name,
            op: Op::Eq,
            values: vec![val.into()],
        })
    }
    pub fn ne(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition(Condition {
            column: self.name,
            op: Op::Ne,
            values: vec![val.into()],
        })
    }
    pub fn lt(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition(Condition {
            column: self.name,
            op: Op::Lt,
            values: vec![val.into()],
        })
    }
    pub fn gt(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition(Condition {
            column: self.name,
            op: Op::Gt,
            values: vec![val.into()],
        })
    }
    pub fn le(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition(Condition {
            column: self.name,
            op: Op::Le,
            values: vec![val.into()],
        })
    }
    pub fn ge(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition(Condition {
            column: self.name,
            op: Op::Ge,
            values: vec![val.into()],
        })
    }
    pub fn like(self, val: impl Into<Value>) -> WhereClause {
        WhereClause::Condition(Condition {
            column: self.name,
            op: Op::Like,
            values: vec![val.into()],
        })
    }
    pub fn is_null(self) -> WhereClause {
        WhereClause::Condition(Condition {
            column: self.name,
            op: Op::IsNull,
            values: vec![],
        })
    }
    pub fn is_not_null(self) -> WhereClause {
        WhereClause::Condition(Condition {
            column: self.name,
            op: Op::IsNotNull,
            values: vec![],
        })
    }
    pub fn is_in(self, vals: Vec<Value>) -> WhereClause {
        WhereClause::Condition(Condition {
            column: self.name,
            op: Op::In,
            values: vals,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(clause: WhereClause) -> (String, Vec<Value>) {
        let mut idx = 0;
        clause.to_sql(&mut idx)
    }

    #[test]
    fn test_eq() {
        let (sql, vals) = build(Expr::col("name").eq("Alice"));
        assert_eq!(sql, "name = $1");
        assert_eq!(vals.len(), 1);
    }

    #[test]
    fn test_ne() {
        let (sql, _) = build(Expr::col("status").ne("banned"));
        assert_eq!(sql, "status != $1");
    }

    #[test]
    fn test_lt_gt_le_ge() {
        assert_eq!(build(Expr::col("age").lt(18)).0, "age < $1");
        assert_eq!(build(Expr::col("age").gt(18)).0, "age > $1");
        assert_eq!(build(Expr::col("age").le(18)).0, "age <= $1");
        assert_eq!(build(Expr::col("age").ge(18)).0, "age >= $1");
    }

    #[test]
    fn test_like() {
        let (sql, _) = build(Expr::col("email").like("%@gmail.com"));
        assert_eq!(sql, "email LIKE $1");
    }

    #[test]
    fn test_is_null_is_not_null() {
        let (sql, vals) = build(Expr::col("deleted_at").is_null());
        assert_eq!(sql, "deleted_at IS NULL");
        assert!(vals.is_empty());

        let (sql, vals) = build(Expr::col("email").is_not_null());
        assert_eq!(sql, "email IS NOT NULL");
        assert!(vals.is_empty());
    }

    #[test]
    fn test_is_in() {
        let (sql, vals) = build(Expr::col("role").is_in(vec![
            Value::from("admin"),
            Value::from("owner"),
        ]));
        assert_eq!(sql, "role IN ($1, $2)");
        assert_eq!(vals.len(), 2);
    }

    #[test]
    fn test_and_chain() {
        let clause = Expr::col("a").eq(1)
            .and(Expr::col("b").eq(2))
            .and(Expr::col("c").eq(3));
        let (sql, vals) = build(clause);
        assert_eq!(sql, "(a = $1 AND b = $2 AND c = $3)");
        assert_eq!(vals.len(), 3);
    }

    #[test]
    fn test_or_chain() {
        let clause = Expr::col("x").eq(1)
            .or(Expr::col("y").eq(2));
        let (sql, vals) = build(clause);
        assert_eq!(sql, "(x = $1 OR y = $2)");
        assert_eq!(vals.len(), 2);
    }

    #[test]
    fn test_nested_and_or() {
        let clause = Expr::col("a").eq(1)
            .and(Expr::col("b").eq(2).or(Expr::col("c").eq(3)));
        let (sql, vals) = build(clause);
        assert_eq!(sql, "(a = $1 AND (b = $2 OR c = $3))");
        assert_eq!(vals.len(), 3);
    }
}
