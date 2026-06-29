use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Versions::Table)
                    .add_column(ColumnDef::new(Versions::ReleaseNotes).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Versions::Table)
                    .drop_column(Versions::ReleaseNotes)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Versions {
    Table,
    ReleaseNotes,
}
