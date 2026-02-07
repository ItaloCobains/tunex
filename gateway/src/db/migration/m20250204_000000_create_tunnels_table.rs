use tunex_query::Migration;

pub struct CreateTunnelsTable;

impl Migration for CreateTunnelsTable {
    fn version(&self) -> i64 {
        20250204_000000
    }

    fn name(&self) -> &str {
        "create_tunnels_table"
    }

    fn up_sql(&self) -> &str {
        "CREATE TABLE IF NOT EXISTS tunnels (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            ip VARCHAR NOT NULL,
            created_at TIMESTAMP,
            updated_at TIMESTAMP
        )"
    }

    fn down_sql(&self) -> &str {
        "DROP TABLE IF EXISTS tunnels"
    }
}
