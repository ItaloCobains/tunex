use sqlx::postgres::PgArguments;

use crate::builder::expr::WhereClause;
use crate::builder::join::Join;
use crate::builder::order::{OrderBy, OrderDir};
use crate::pool::Pool;
use crate::value::Value;

#[derive(Debug, Clone)]
pub struct SelectBuilder {
    table: String,
    columns: Vec<String>,
    filters: Vec<WhereClause>,
    joins: Vec<Join>,
    orders: Vec<OrderBy>,
    limit_val: Option<i64>,
    offset_val: Option<i64>,
}

impl SelectBuilder {
    pub fn table(name: &str) -> Self {
        Self {
            table: name.to_string(),
            columns: vec![],
            filters: vec![],
            joins: vec![],
            orders: vec![],
            limit_val: None,
            offset_val: None,
        }
    }

    pub fn columns(mut self, cols: &[&str]) -> Self {
        self.columns = cols.iter().map(|c| c.to_string()).collect();
        self
    }

    pub fn filter(mut self, clause: WhereClause) -> Self {
        self.filters.push(clause);
        self
    }

    pub fn join(mut self, j: Join) -> Self {
        self.joins.push(j);
        self
    }

    pub fn order_by(mut self, col: &str, dir: OrderDir) -> Self {
        self.orders.push(OrderBy {
            column: col.to_string(),
            direction: dir,
        });
        self
    }

    pub fn limit(mut self, n: i64) -> Self {
        self.limit_val = Some(n);
        self
    }

    pub fn offset(mut self, n: i64) -> Self {
        self.offset_val = Some(n);
        self
    }

    pub fn build(&self) -> (String, Vec<Value>) {
        let cols = if self.columns.is_empty() {
            "*".to_string()
        } else {
            self.columns.join(", ")
        };

        let mut sql = format!("SELECT {} FROM {}", cols, self.table);
        let mut values: Vec<Value> = Vec::new();
        let mut idx: i32 = 0;

        for j in &self.joins {
            let kind = match j.kind {
                crate::builder::join::JoinKind::Inner => "INNER JOIN",
                crate::builder::join::JoinKind::Left => "LEFT JOIN",
                crate::builder::join::JoinKind::Right => "RIGHT JOIN",
            };
            sql.push_str(&format!(
                " {} {} ON {} = {}",
                kind, j.table, j.on_left, j.on_right
            ));
        }

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

        if !self.orders.is_empty() {
            let order_parts: Vec<String> = self
                .orders
                .iter()
                .map(|o| {
                    let dir = match o.direction {
                        OrderDir::Asc => "ASC",
                        OrderDir::Desc => "DESC",
                    };
                    format!("{} {}", o.column, dir)
                })
                .collect();
            sql.push_str(&format!(" ORDER BY {}", order_parts.join(", ")));
        }

        if let Some(n) = self.limit_val {
            sql.push_str(&format!(" LIMIT {}", n));
        }
        if let Some(n) = self.offset_val {
            sql.push_str(&format!(" OFFSET {}", n));
        }

        (sql, values)
    }

    pub async fn fetch_all(
        &self,
        pool: &Pool,
    ) -> crate::Result<Vec<sqlx::postgres::PgRow>> {
        let (sql, values) = self.build();
        let mut args = PgArguments::default();
        for v in &values {
            v.bind_to(&mut args);
        }
        let rows = sqlx::query_with(&sql, args)
            .fetch_all(pool.inner())
            .await?;
        Ok(rows)
    }

    pub async fn fetch_one(
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

    pub async fn fetch_optional(
        &self,
        pool: &Pool,
    ) -> crate::Result<Option<sqlx::postgres::PgRow>> {
        let (sql, values) = self.build();
        let mut args = PgArguments::default();
        for v in &values {
            v.bind_to(&mut args);
        }
        let row = sqlx::query_with(&sql, args)
            .fetch_optional(pool.inner())
            .await?;
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::expr::Expr;

    #[test]
    fn test_simple_select() {
        let (sql, values) = SelectBuilder::table("users")
            .columns(&["id", "name"])
            .build();
        assert_eq!(sql, "SELECT id, name FROM users");
        assert!(values.is_empty());
    }

    #[test]
    fn test_select_star() {
        let (sql, _) = SelectBuilder::table("users").build();
        assert_eq!(sql, "SELECT * FROM users");
    }

    #[test]
    fn test_select_with_filter() {
        let (sql, values) = SelectBuilder::table("users")
            .columns(&["id", "name"])
            .filter(Expr::col("email").eq("alice@example.com"))
            .build();
        assert_eq!(sql, "SELECT id, name FROM users WHERE email = $1");
        assert_eq!(values.len(), 1);
    }

    #[test]
    fn test_select_with_limit_offset() {
        let (sql, _) = SelectBuilder::table("users")
            .columns(&["id"])
            .limit(10)
            .offset(20)
            .build();
        assert_eq!(sql, "SELECT id FROM users LIMIT 10 OFFSET 20");
    }

    #[test]
    fn test_select_with_order() {
        let (sql, _) = SelectBuilder::table("users")
            .columns(&["id"])
            .order_by("created_at", OrderDir::Desc)
            .build();
        assert_eq!(sql, "SELECT id FROM users ORDER BY created_at DESC");
    }

    #[test]
    fn test_select_multiple_filters() {
        let (sql, values) = SelectBuilder::table("users")
            .filter(Expr::col("name").eq("Alice"))
            .filter(Expr::col("id").gt(5))
            .build();
        assert_eq!(sql, "SELECT * FROM users WHERE name = $1 AND id > $2");
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_select_inner_join() {
        let (sql, _) = SelectBuilder::table("tunnels")
            .columns(&["tunnels.id", "users.name"])
            .join(crate::builder::join::Join {
                kind: crate::builder::join::JoinKind::Inner,
                table: "users".to_string(),
                on_left: "tunnels.user_id".to_string(),
                on_right: "users.id".to_string(),
            })
            .build();
        assert_eq!(
            sql,
            "SELECT tunnels.id, users.name FROM tunnels INNER JOIN users ON tunnels.user_id = users.id"
        );
    }

    #[test]
    fn test_select_left_join_with_filter() {
        let (sql, values) = SelectBuilder::table("users")
            .columns(&["users.id", "tunnels.ip"])
            .join(crate::builder::join::Join {
                kind: crate::builder::join::JoinKind::Left,
                table: "tunnels".to_string(),
                on_left: "users.id".to_string(),
                on_right: "tunnels.user_id".to_string(),
            })
            .filter(Expr::col("users.id").eq(1))
            .build();
        assert_eq!(
            sql,
            "SELECT users.id, tunnels.ip FROM users LEFT JOIN tunnels ON users.id = tunnels.user_id WHERE users.id = $1"
        );
        assert_eq!(values.len(), 1);
    }

    #[test]
    fn test_select_and_or_expressions() {
        let clause = Expr::col("age").gt(18).and(Expr::col("active").eq(true));
        let (sql, values) = SelectBuilder::table("users")
            .filter(clause)
            .build();
        assert_eq!(sql, "SELECT * FROM users WHERE (age > $1 AND active = $2)");
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_select_or_expression() {
        let clause = Expr::col("role").eq("admin").or(Expr::col("role").eq("owner"));
        let (sql, values) = SelectBuilder::table("users")
            .filter(clause)
            .build();
        assert_eq!(sql, "SELECT * FROM users WHERE (role = $1 OR role = $2)");
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_select_multiple_orders() {
        let (sql, _) = SelectBuilder::table("users")
            .order_by("name", OrderDir::Asc)
            .order_by("created_at", OrderDir::Desc)
            .build();
        assert_eq!(sql, "SELECT * FROM users ORDER BY name ASC, created_at DESC");
    }

    #[test]
    fn test_select_full_query() {
        let (sql, values) = SelectBuilder::table("tunnels")
            .columns(&["tunnels.id", "tunnels.ip", "users.name"])
            .join(crate::builder::join::Join {
                kind: crate::builder::join::JoinKind::Inner,
                table: "users".to_string(),
                on_left: "tunnels.user_id".to_string(),
                on_right: "users.id".to_string(),
            })
            .filter(Expr::col("users.name").eq("Alice"))
            .order_by("tunnels.created_at", OrderDir::Desc)
            .limit(5)
            .offset(0)
            .build();
        assert_eq!(
            sql,
            "SELECT tunnels.id, tunnels.ip, users.name FROM tunnels INNER JOIN users ON tunnels.user_id = users.id WHERE users.name = $1 ORDER BY tunnels.created_at DESC LIMIT 5 OFFSET 0"
        );
        assert_eq!(values.len(), 1);
    }
}
