use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Versions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Versions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Versions::ProjectId).integer().not_null())
                    .col(ColumnDef::new(Versions::N).integer().not_null())
                    .col(ColumnDef::new(Versions::HtmlPath).string().not_null())
                    .col(
                        ColumnDef::new(Versions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_versions_project_id")
                            .from(Versions::Table, Versions::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Backstop d'intégrité : un seul `n` par projet (compteur v1, v2…).
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_versions_project_n")
                    .table(Versions::Table)
                    .col(Versions::ProjectId)
                    .col(Versions::N)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Versions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Versions {
    Table,
    Id,
    ProjectId,
    N,
    HtmlPath,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
}
