pub use super::_entities::versions::{ActiveModel, Entity, Model};
use sea_orm::entity::prelude::*;
pub type Versions = Entity;

impl ActiveModelBehavior for ActiveModel {}

// implement your read-oriented logic here
impl Model {}

// implement your write-oriented logic here
impl ActiveModel {}

// implement your custom finders, selectors oriented logic here
impl Entity {}
