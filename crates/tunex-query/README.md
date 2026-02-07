# tunex-query

Lightweight query builder for PostgreSQL, built on top of [sqlx](https://github.com/launchbadge/sqlx).

## Setup

```rust
let pool = tunex_query::Pool::connect("postgres://user:pass@localhost/db").await?;
```

## Select

```rust
use tunex_query::{SelectBuilder, Expr, OrderDir};

let rows = SelectBuilder::table("users")
    .columns(&["id", "name", "email"])
    .fetch_all(&pool).await?;

let rows = SelectBuilder::table("users")
    .columns(&["id", "name"])
    .filter(Expr::col("email").eq("alice@example.com"))
    .order_by("created_at", OrderDir::Desc)
    .limit(10)
    .offset(20)
    .fetch_all(&pool).await?;

let row = SelectBuilder::table("users")
    .filter(Expr::col("id").eq(1))
    .fetch_one(&pool).await?;

let maybe = SelectBuilder::table("users")
    .filter(Expr::col("id").eq(999))
    .fetch_optional(&pool).await?;
```

## Insert

```rust
use tunex_query::InsertBuilder;

InsertBuilder::table("users")
    .set("name", "Alice")
    .set("email", "alice@example.com")
    .set("password", "hashed_pw")
    .execute(&pool).await?;

let row = InsertBuilder::table("users")
    .set("name", "Bob")
    .set("email", "bob@example.com")
    .set("password", "hashed_pw")
    .returning(&["id"])
    .execute_returning(&pool).await?;
```

## Update

```rust
use tunex_query::{UpdateBuilder, Expr};

UpdateBuilder::table("tunnels")
    .set("ip", "10.0.0.1")
    .filter(Expr::col("id").eq(7))
    .execute(&pool).await?;
```

## Delete

```rust
use tunex_query::{DeleteBuilder, Expr};

DeleteBuilder::table("tunnels")
    .filter(Expr::col("user_id").eq(42))
    .execute(&pool).await?;
```

## Expressions

```rust
use tunex_query::{Expr, Value};

Expr::col("age").gt(18)
Expr::col("name").like("%alice%")
Expr::col("deleted_at").is_null()
Expr::col("status").is_in(vec![Value::from("active"), Value::from("pending")])

let clause = Expr::col("age").gt(18).and(Expr::col("active").eq(true));
let clause = Expr::col("role").eq("admin").or(Expr::col("role").eq("owner"));
```

## Reading rows

Use the `RowExt` trait on `PgRow`:

```rust
use tunex_query::RowExt;

let row = SelectBuilder::table("users")
    .filter(Expr::col("id").eq(1))
    .fetch_one(&pool).await?;

let id: i32 = row.get_i32("id");
let name: String = row.get_string("name");
let created: Option<NaiveDateTime> = row.get_opt_timestamp("created_at");
```

## Migrations

```rust
use tunex_query::{Migration, Migrator};

struct CreateUsersTable;

impl Migration for CreateUsersTable {
    fn version(&self) -> i64 { 20250203_000000 }
    fn name(&self) -> &str { "create_users_table" }
    fn up_sql(&self) -> &str { "CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR NOT NULL)" }
    fn down_sql(&self) -> &str { "DROP TABLE users" }
}

let migrator = Migrator::new(vec![Box::new(CreateUsersTable)]);
migrator.run(pool.inner()).await?;
```

Migrations are tracked in a `_tunex_migrations` table and run inside transactions.

## SQL inspection

Every builder has a `build()` method that returns the raw SQL and params without executing:

```rust
let (sql, params) = SelectBuilder::table("users")
    .filter(Expr::col("id").eq(1))
    .build();
// sql = "SELECT * FROM users WHERE id = $1"
// params = [Value::Int(1)]
```
