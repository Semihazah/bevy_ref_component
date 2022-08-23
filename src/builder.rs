use bevy::prelude::{Commands, Component, Entity, FromWorld, World};

use crate::{EditFn, InsertFn, RefCompExt, RefCompHandle, RefCompServer};

pub struct RefCompBuilder<T: Component> {
    entity: Entity,
    insert_fn: Option<InsertFn<T>>,
    edit_fn: Option<EditFn<T>>,
}

impl<T: Component> RefCompBuilder<T> {
    pub fn with_edit_fn(mut self, edit_fn: EditFn<T>) -> Self {
        self.edit_fn = Some(edit_fn);
        self
    }
}
pub trait RefCompBuilderExt<T: Component> {
    fn new(entity: Entity, insert_fn: InsertFn<T>) -> Self;
    fn build(
        &mut self,
        commands: &mut Commands,
        ref_comp_server: &mut RefCompServer,
    ) -> RefCompHandle<T>;
    fn build_world(&mut self, world: &mut World) -> RefCompHandle<T>;
}

impl<T: Component> RefCompBuilderExt<T> for RefCompBuilder<T> {
    fn new(entity: Entity, insert_fn: InsertFn<T>) -> Self {
        RefCompBuilder {
            entity,
            insert_fn: Some(insert_fn),
            edit_fn: None,
        }
    }

    fn build(
        &mut self,
        commands: &mut Commands,
        ref_comp_server: &mut RefCompServer,
    ) -> RefCompHandle<T> {
        let insert_fn = self
            .insert_fn
            .expect("Attempted to build without insert fn!");
        ref_comp_server.insert_ref_comp(commands, self.entity, insert_fn, self.edit_fn)
    }

    fn build_world(&mut self, world: &mut World) -> RefCompHandle<T> {
        let insert_fn = self
            .insert_fn
            .expect("Attempted to build without insert fn!");
        world.insert_ref_comp(self.entity, insert_fn, self.edit_fn)
    }
}

pub trait RefCompBuilderFromWorldExt<T: Component + FromWorld> {
    fn new(entity: Entity) -> Self;
    fn with_insert_fn(self, insert_fn: InsertFn<T>) -> Self;
    fn build(
        &mut self,
        commands: &mut Commands,
        ref_comp_server: &mut RefCompServer,
    ) -> RefCompHandle<T>;
    fn build_world(&mut self, world: &mut World) -> RefCompHandle<T>;
}

impl<T: Component + FromWorld> RefCompBuilderFromWorldExt<T> for RefCompBuilder<T> {
    fn new(entity: Entity) -> Self {
        RefCompBuilder {
            entity,
            insert_fn: None,
            edit_fn: None,
        }
    }

    fn with_insert_fn(mut self, insert_fn: InsertFn<T>) -> Self {
        self.insert_fn = Some(insert_fn);
        self
    }

    fn build(
        &mut self,
        commands: &mut Commands,
        ref_comp_server: &mut RefCompServer,
    ) -> RefCompHandle<T> {
        match self.insert_fn {
            Some(i) => ref_comp_server.insert_ref_comp(commands, self.entity, i, self.edit_fn),
            None => ref_comp_server.insert_ref_comp_fw(commands, self.entity, self.edit_fn),
        }
    }

    fn build_world(&mut self, world: &mut World) -> RefCompHandle<T> {
        match self.insert_fn {
            Some(i) => world.insert_ref_comp(self.entity, i, self.edit_fn),
            None => world.insert_ref_comp_from_world(self.entity, self.edit_fn),
        }
    }
}
