use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tunnels::Table)
                    .if_not_exists()
                    .col(pk_auto(Tunnels::Id))
                    .col(integer(Tunnels::UserId).not_null())
                    .col(string(Tunnels::Ip).not_null())
                    .col(timestamp(Tunnels::CreatedAt))
                    .col(timestamp(Tunnels::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tunnels_user_id")
                            .from(Tunnels::Table, Tunnels::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tunnels::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Tunnels {
    Table,
    Id,
    UserId,
    Ip,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
