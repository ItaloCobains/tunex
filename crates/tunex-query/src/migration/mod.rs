mod traits;

pub use traits::Migration;

use sqlx::PgPool;

pub struct Migrator {
    migrations: Vec<Box<dyn Migration>>,
}

impl Migrator {
    pub fn new(migrations: Vec<Box<dyn Migration>>) -> Self {
        Self { migrations }
    }

    pub async fn run(&self, pool: &PgPool) -> crate::Result<()> {
        self.ensure_tracking_table(pool).await?;

        for migration in &self.migrations {
            if !self.is_applied(pool, migration.version()).await? {
                println!(
                    "Running migration {} (v{})...",
                    migration.name(),
                    migration.version()
                );
                let mut tx = pool.begin().await?;
                sqlx::query(migration.up_sql())
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        crate::Error::Migration(format!(
                            "Failed to run migration {}: {}",
                            migration.name(),
                            e
                        ))
                    })?;
                sqlx::query(
                    "INSERT INTO _tunex_migrations (version, name) VALUES ($1, $2)",
                )
                .bind(migration.version())
                .bind(migration.name())
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
            }
        }

        Ok(())
    }

    async fn ensure_tracking_table(&self, pool: &PgPool) -> crate::Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _tunex_migrations (
                version BIGINT PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TIMESTAMP NOT NULL DEFAULT NOW()
            )",
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn is_applied(&self, pool: &PgPool, version: i64) -> crate::Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM _tunex_migrations WHERE version = $1")
            .bind(version)
            .fetch_one(pool)
            .await?;
        let count: i64 = sqlx::Row::get(&row, "cnt");
        Ok(count > 0)
    }
}
