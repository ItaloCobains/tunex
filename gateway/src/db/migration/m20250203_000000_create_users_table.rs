use tunex_query::Migration;

pub struct CreateUsersTable;

impl Migration for CreateUsersTable {
    fn version(&self) -> i64 {
        20250203_000000
    }

    fn name(&self) -> &str {
        "create_users_table"
    }

    fn up_sql(&self) -> &str {
        "CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            email VARCHAR NOT NULL UNIQUE,
            password VARCHAR NOT NULL,
            created_at TIMESTAMP
        )"
    }

    fn down_sql(&self) -> &str {
        "DROP TABLE IF EXISTS users"
    }
}
