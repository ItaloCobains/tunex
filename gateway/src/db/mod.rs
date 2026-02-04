//! Database layer: connection, migrations, entities.

mod entity;
mod migration;

pub use entity::{Tunnel, TunnelModel, User, UserModel};
pub use migration::Migrator;

use sea_orm::{Database, DbErr};
use sea_orm_migration::MigratorTrait;

/// Connect to the database and run pending migrations.
pub async fn connect_and_migrate(database_url: &str) -> Result<sea_orm::DatabaseConnection, DbErr> {
    let db = Database::connect(database_url).await?;
    Migrator::up(&db, None).await?;
    Ok(db)
}
