mod m20250203_000000_create_users_table;
mod m20250204_000000_create_tunnels_table;

use async_trait::async_trait;
use sea_orm_migration::MigratorTrait;

pub struct Migrator;

#[async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        vec![
            Box::new(m20250203_000000_create_users_table::Migration),
            Box::new(m20250204_000000_create_tunnels_table::Migration),
        ]
    }
}
