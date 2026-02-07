use sqlx::postgres::PgArguments;

use crate::pool::Pool;
use crate::value::Value;

#[derive(Debug, Clone)]
pub struct InsertBuilder {
    table: String,
    columns: Vec<String>,
    values: Vec<Value>,
    returning: Vec<String>,
}

impl InsertBuilder {
    pub fn table(name: &str) -> Self {
        Self {
            table: name.to_string(),
            columns: vec![],
            values: vec![],
            returning: vec![],
        }
    }

    pub fn set(mut self, col: &str, val: impl Into<Value>) -> Self {
        self.columns.push(col.to_string());
        self.values.push(val.into());
        self
    }

    pub fn returning(mut self, cols: &[&str]) -> Self {
        self.returning = cols.iter().map(|c| c.to_string()).collect();
        self
    }

    pub fn build(&self) -> (String, Vec<Value>) {
        let cols = self.columns.join(", ");
        let placeholders: Vec<String> = (1..=self.columns.len())
            .map(|i| format!("${}", i))
            .collect();
        let mut sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table,
            cols,
            placeholders.join(", ")
        );

        if !self.returning.is_empty() {
            sql.push_str(&format!(" RETURNING {}", self.returning.join(", ")));
        }

        (sql, self.values.clone())
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

    pub async fn execute_returning(
        &self,
        pool: &Pool,
    ) -> crate::Result<sqlx::postgres::PgRow> {
        let (sql, values) = self.build();
        let mut args = PgArguments::default();
        for v in &values {
            v.bind_to(&mut args);
        }
        let row = sqlx::query_with(&sql, args)
            .fetch_one(pool.inner())
            .await?;
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_insert() {
        let (sql, values) = InsertBuilder::table("users")
            .set("name", "Alice")
            .set("email", "alice@example.com")
            .build();
        assert_eq!(
            sql,
            "INSERT INTO users (name, email) VALUES ($1, $2)"
        );
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_insert_with_returning() {
        let (sql, values) = InsertBuilder::table("users")
            .set("name", "Bob")
            .returning(&["id"])
            .build();
        assert_eq!(
            sql,
            "INSERT INTO users (name) VALUES ($1) RETURNING id"
        );
        assert_eq!(values.len(), 1);
    }
}
