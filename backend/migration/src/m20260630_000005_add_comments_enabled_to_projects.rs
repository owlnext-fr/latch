use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Projects::Table)
                    .add_column(
                        ColumnDef::new(Projects::CommentsEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        // Backfill sécurité-aware : les projets existants suivent leur code d'accès.
        manager
            .get_connection()
            .execute_unprepared("UPDATE projects SET comments_enabled = code_enabled")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Projects::Table)
                    .drop_column(Projects::CommentsEnabled)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    CommentsEnabled,
}
