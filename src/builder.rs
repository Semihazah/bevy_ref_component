use bevy::prelude::{Commands, Component, Entity, FromWorld, World};

use crate::{EditFn, InsertFn, RefCompExt, RefCompHandle, RefCompServer};

pub struct RefCompBuilder<T: Component> {
    entity: Entity,
    insert_fn: InsertFn<T>,
    edit_fn: Option<EditFn<T>>,
}

impl<T: Component> RefCompBuilder<T> {
    pub fn new(entity: Entity, insert_fn: InsertFn<T>) -> Self {
        RefCompBuilder {
            entity,
            insert_fn,
            edit_fn: None,
        }
    }

    pub fn with_edit_fn(mut self, edit_fn: EditFn<T>) -> Self {
        self.edit_fn = Some(edit_fn);
        self
    }

    pub fn with_insert_fn(mut self, insert_fn: InsertFn<T>) -> Self {
        self.insert_fn = insert_fn;
        self
    }

    pub fn build(
        &mut self,
        commands: &mut Commands,
        ref_comp_server: &mut RefCompServer,
    ) -> RefCompHandle<T> {
        ref_comp_server.insert_ref_comp(commands, self.entity, self.insert_fn, self.edit_fn)
    }

    pub fn build_world(&mut self, world: &mut World) -> RefCompHandle<T> {
        world.insert_ref_comp(self.entity, self.insert_fn, self.edit_fn)
    }
}

impl<T: Component + FromWorld> RefCompBuilder<T> {
    pub fn new_fw(entity: Entity) -> Self {
        RefCompBuilder {
            entity,
            insert_fn: |world: &mut World, _entity| T::from_world(world),
            edit_fn: None,
        }
    }
}
pub trait RefCompBuilderExt<T: Component> {
    fn new(entity: Entity, insert_fn: InsertFn<T>) -> Self;
}

impl<T: Component> RefCompBuilderExt<T> for RefCompBuilder<T> {
    fn new(entity: Entity, insert_fn: InsertFn<T>) -> Self {
        RefCompBuilder {
            entity,
            insert_fn,
            edit_fn: None,
        }
    }
}
