//! Database layer: connection, migrations, models.

mod migration;
pub mod models;

use tunex_query::{Migrator, Pool};

/// Connect to the database and run pending migrations.
pub async fn connect_and_migrate(database_url: &str) -> tunex_query::Result<Pool> {
    let pool = Pool::connect(database_url).await?;

    let migrator = Migrator::new(vec![
        Box::new(migration::CreateUsersTable),
        Box::new(migration::CreateTunnelsTable),
    ]);
    migrator.run(pool.inner()).await?;

    Ok(pool)
}
