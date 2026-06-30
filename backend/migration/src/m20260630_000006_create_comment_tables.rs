use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CommentPins::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CommentPins::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CommentPins::VersionId).integer().not_null())
                    .col(ColumnDef::new(CommentPins::OwnerToken).string().not_null())
                    .col(ColumnDef::new(CommentPins::Anchor).text().not_null())
                    .col(
                        ColumnDef::new(CommentPins::Status)
                            .string()
                            .not_null()
                            .default("open"),
                    )
                    .col(
                        ColumnDef::new(CommentPins::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CommentPins::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CommentPins::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_comment_pins_version_id")
                            .from(CommentPins::Table, CommentPins::VersionId)
                            .to(Versions::Table, Versions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_comment_pins_version_id")
                    .table(CommentPins::Table)
                    .col(CommentPins::VersionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Comments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Comments::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Comments::PinId).integer().not_null())
                    .col(ColumnDef::new(Comments::OwnerToken).string().not_null())
                    .col(ColumnDef::new(Comments::AuthorName).string().not_null())
                    .col(ColumnDef::new(Comments::Body).text().not_null())
                    .col(
                        ColumnDef::new(Comments::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Comments::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Comments::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_comments_pin_id")
                            .from(Comments::Table, Comments::PinId)
                            .to(CommentPins::Table, CommentPins::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_comments_pin_id")
                    .table(Comments::Table)
                    .col(Comments::PinId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Comments::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CommentPins::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CommentPins {
    Table,
    Id,
    VersionId,
    OwnerToken,
    Anchor,
    Status,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum Comments {
    Table,
    Id,
    PinId,
    OwnerToken,
    AuthorName,
    Body,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum Versions {
    Table,
    Id,
}
