pub use super::_entities::comments::{ActiveModel, Entity, Model};
use sea_orm::entity::prelude::*;
pub type Comments = Entity;

impl ActiveModelBehavior for ActiveModel {}

// implement your read-oriented logic here
impl Model {}

// implement your write-oriented logic here
impl ActiveModel {}

// implement your custom finders, selectors oriented logic here
impl Entity {}
