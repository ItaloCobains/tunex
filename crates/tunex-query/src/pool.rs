use sqlx::postgres::{PgPool, PgPoolOptions};

#[derive(Clone)]
pub struct Pool {
    inner: PgPool,
}

impl Pool {
    pub async fn connect(database_url: &str) -> crate::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { inner: pool })
    }

    pub fn inner(&self) -> &PgPool {
        &self.inner
    }
}
