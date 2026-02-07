use sqlx::postgres::PgArguments;

use crate::builder::expr::WhereClause;
use crate::pool::Pool;
use crate::value::Value;

#[derive(Debug, Clone)]
pub struct DeleteBuilder {
    table: String,
    filters: Vec<WhereClause>,
}

impl DeleteBuilder {
    pub fn table(name: &str) -> Self {
        Self {
            table: name.to_string(),
            filters: vec![],
        }
    }

    pub fn filter(mut self, clause: WhereClause) -> Self {
        self.filters.push(clause);
        self
    }

    pub fn build(&self) -> (String, Vec<Value>) {
        let mut sql = format!("DELETE FROM {}", self.table);
        let mut values: Vec<Value> = Vec::new();
        let mut idx: i32 = 0;

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
    fn test_simple_delete() {
        let (sql, values) = DeleteBuilder::table("tunnels")
            .filter(Expr::col("user_id").eq(42))
            .build();
        assert_eq!(sql, "DELETE FROM tunnels WHERE user_id = $1");
        assert_eq!(values.len(), 1);
    }

    #[test]
    fn test_delete_no_filter() {
        let (sql, values) = DeleteBuilder::table("sessions").build();
        assert_eq!(sql, "DELETE FROM sessions");
        assert!(values.is_empty());
    }

    #[test]
    fn test_delete_multiple_filters() {
        let (sql, values) = DeleteBuilder::table("tunnels")
            .filter(Expr::col("user_id").eq(1))
            .filter(Expr::col("ip").eq("10.0.0.1"))
            .build();
        assert_eq!(
            sql,
            "DELETE FROM tunnels WHERE user_id = $1 AND ip = $2"
        );
        assert_eq!(values.len(), 2);
    }
}
