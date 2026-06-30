#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;

mod m20260624_000001_create_projects;
mod m20260624_000002_create_versions;
mod m20260624_000003_create_sessions;
mod m20260629_000004_add_release_notes_to_versions;
mod m20260630_000005_add_comments_enabled_to_projects;
mod m20260630_000006_create_comment_tables;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260624_000001_create_projects::Migration),
            Box::new(m20260624_000002_create_versions::Migration),
            Box::new(m20260624_000003_create_sessions::Migration),
            Box::new(m20260629_000004_add_release_notes_to_versions::Migration),
            Box::new(m20260630_000005_add_comments_enabled_to_projects::Migration),
            Box::new(m20260630_000006_create_comment_tables::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
