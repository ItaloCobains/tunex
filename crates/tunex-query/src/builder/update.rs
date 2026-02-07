use sqlx::postgres::PgArguments;

use crate::builder::expr::WhereClause;
use crate::pool::Pool;
use crate::value::Value;

#[derive(Debug, Clone)]
pub struct UpdateBuilder {
    table: String,
    sets: Vec<(String, Value)>,
    filters: Vec<WhereClause>,
    returning: Vec<String>,
}

impl UpdateBuilder {
    pub fn table(name: &str) -> Self {
        Self {
            table: name.to_string(),
            sets: vec![],
            filters: vec![],
            returning: vec![],
        }
    }

    pub fn set(mut self, col: &str, val: impl Into<Value>) -> Self {
        self.sets.push((col.to_string(), val.into()));
        self
    }

    pub fn filter(mut self, clause: WhereClause) -> Self {
        self.filters.push(clause);
        self
    }

    pub fn returning(mut self, cols: &[&str]) -> Self {
        self.returning = cols.iter().map(|c| c.to_string()).collect();
        self
    }

    pub fn build(&self) -> (String, Vec<Value>) {
        let mut idx: i32 = 0;
        let mut values: Vec<Value> = Vec::new();

        let set_parts: Vec<String> = self
            .sets
            .iter()
            .map(|(col, val)| {
                idx += 1;
                values.push(val.clone());
                format!("{} = ${}", col, idx)
            })
            .collect();

        let mut sql = format!("UPDATE {} SET {}", self.table, set_parts.join(", "));

        if !self.filters.is_empty() {
            let mut where_parts = Vec::new();
            for f in &self.filters {
                let (s, v) = f.to_sql(&mut idx);
                where_parts.push(s);
                values.extend(v);
            }
            sql.push_str(" WHERE ");
            sql.push_str(&where_parts.join(" AND "));
        }

        if !self.returning.is_empty() {
            sql.push_str(&format!(" RETURNING {}", self.returning.join(", ")));
        }

        (sql, values)
    }

    pub async fn execute(&self, pool: &Pool) -> crate::Result<u64> {
        let (sql, values) = self.build();
        let mut args = PgArguments::default();
        for v in &values {
            v.bind_to(&mut args);
        }
        let result = sqlx::query_with(&sql, args)
            .execute(pool.inner())
            .await?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::expr::Expr;

    #[test]
    fn test_simple_update() {
        let (sql, values) = UpdateBuilder::table("tunnels")
            .set("ip", "10.0.0.1")
            .filter(Expr::col("id").eq(7))
            .build();
        assert_eq!(sql, "UPDATE tunnels SET ip = $1 WHERE id = $2");
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_update_multiple_sets() {
        let (sql, values) = UpdateBuilder::table("users")
            .set("name", "Alice")
            .set("email", "alice@new.com")
            .filter(Expr::col("id").eq(1))
            .build();
        assert_eq!(
            sql,
            "UPDATE users SET name = $1, email = $2 WHERE id = $3"
        );
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_update_with_returning() {
        let (sql, _) = UpdateBuilder::table("users")
            .set("name", "Bob")
            .returning(&["id", "name"])
            .build();
        assert_eq!(
            sql,
            "UPDATE users SET name = $1 RETURNING id, name"
        );
    }
}
