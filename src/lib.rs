use std::{any::type_name, marker::PhantomData};

use bevy::{
    ecs::reflect::ReflectComponent,
    prelude::{
        App, Commands, Component, CoreStage, Entity, FromWorld, Mut, Plugin, Query, ResMut,
        StageLabel, SystemStage, World, Resource,
    },
    reflect::{FromReflect, Reflect, ReflectDeserialize, ReflectSerialize},
    utils::HashMap,
};
use crossbeam_channel::{Receiver, Sender};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod builder;
pub use builder::RefCompBuilder;

type InsertFn<T> = fn(&mut World, Entity) -> T;
type EditFn<T> = fn(&mut World, Entity, &mut T);

pub struct RefCompPlugin;

#[derive(StageLabel)]
pub struct DespawnStage;

impl Plugin for RefCompPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RefCompServer>()
            .add_stage_after(CoreStage::Update, DespawnStage, SystemStage::parallel())
            .add_system_to_stage(DespawnStage, delete_unreferenced_components);
    }
}
// *****************************************************************************************
// Resources
// *****************************************************************************************
#[derive(Default, Resource)]
pub struct RefCompServer {
    channel: RefChangeChannel,
    ref_counts: HashMap<RefCompHandleId, usize>,
    comp_spawner: HashMap<String, RefComponentSpawner>,
}

impl RefCompServer {
    pub fn get_handle<T: Component, I: Into<RefCompHandleId>>(&self, id: I) -> RefCompHandle<T> {
        let sender = self.channel.sender.clone();
        RefCompHandle::strong(id.into(), sender)
    }

    pub fn get_handle_untyped<I: Into<RefCompHandleId>>(&self, id: I) -> RefCompHandleUntyped {
        let sender = self.channel.sender.clone();
        RefCompHandleUntyped::strong(id.into(), sender)
    }

    fn inner_insert_ref_comp_from_world<T: Component + FromWorld>(
        &mut self,
        world: &mut World,
        entity: Entity,
        edit_fn: Option<EditFn<T>>,
    ) -> RefCompHandle<T> {
        let handle_id = RefCompHandleId::new::<T>(entity);

        if !self.comp_spawner.contains_key(&handle_id.type_id) {
            self.comp_spawner.insert(
                type_name::<T>().to_string(),
                RefComponentSpawner {
                    delete: delete_component::<T>,
                },
            );
        }

        match world.entity(handle_id.entity).contains::<T>() {
            true => {
                if let Some(edit_fn) = edit_fn {
                    if let Some(mut comp) = world.entity_mut(entity).remove::<T>() {
                        edit_fn(world, entity, &mut comp);
                        world.entity_mut(entity).insert(comp);
                    }
                }
            }
            false => {
                let comp = T::from_world(world);
                world.entity_mut(handle_id.entity).insert(comp);
            }
        }
        self.get_handle(handle_id)
    }

    fn inner_insert_ref_comp<T: Component>(
        &mut self,
        world: &mut World,
        entity: Entity,
        insert_fn: InsertFn<T>,
        edit_fn: Option<EditFn<T>>,
    ) -> RefCompHandle<T> {
        let handle_id = RefCompHandleId::new::<T>(entity);

        if !self.comp_spawner.contains_key(&handle_id.type_id) {
            self.comp_spawner.insert(
                type_name::<T>().to_string(),
                RefComponentSpawner {
                    delete: delete_component::<T>,
                },
            );
        }

        match world.entity(handle_id.entity).contains::<T>() {
            true => {
                if let Some(edit_fn) = edit_fn {
                    if let Some(mut comp) = world.entity_mut(entity).remove::<T>() {
                        edit_fn(world, entity, &mut comp);
                        world.entity_mut(entity).insert(comp);
                    }
                }
            }
            false => {
                let comp = insert_fn(world, handle_id.entity);
                world.entity_mut(handle_id.entity).insert(comp);
            }
        }

        self.get_handle(handle_id)
    }

    pub fn insert_ref_comp_fw<T: Component + FromWorld>(
        &mut self,
        commands: &mut Commands,
        entity: Entity,
        edit_fn: Option<EditFn<T>>,
    ) -> RefCompHandle<T> {
        let handle_id = RefCompHandleId::new::<T>(entity);

        if !self.comp_spawner.contains_key(&handle_id.type_id) {
            self.comp_spawner.insert(
                type_name::<T>().to_string(),
                RefComponentSpawner {
                    delete: delete_component::<T>,
                },
            );
        }

        commands.add(move |world: &mut World| {
            match world.entity(handle_id.entity).contains::<T>() {
                true => {
                    if let Some(edit_fn) = edit_fn {
                        if let Some(mut comp) = world.entity_mut(entity).remove::<T>() {
                            edit_fn(world, entity, &mut comp);
                            world.entity_mut(entity).insert(comp);
                        }
                    }
                }
                false => {
                    let comp = T::from_world(world);
                    world.entity_mut(handle_id.entity).insert(comp);
                }
            }
        });

        self.get_handle(handle_id)
    }

    pub fn insert_ref_comp<T: Component>(
        &mut self,
        commands: &mut Commands,
        entity: Entity,
        insert_fn: InsertFn<T>,
        edit_fn: Option<EditFn<T>>,
    ) -> RefCompHandle<T> {
        let handle_id = RefCompHandleId::new::<T>(entity);

        if !self.comp_spawner.contains_key(&handle_id.type_id) {
            self.comp_spawner.insert(
                type_name::<T>().to_string(),
                RefComponentSpawner {
                    delete: delete_component::<T>,
                },
            );
        }

        commands.add(move |world: &mut World| {
            match world.entity(handle_id.entity).contains::<T>() {
                true => {
                    if let Some(edit_fn) = edit_fn {
                        if let Some(mut comp) = world.entity_mut(entity).remove::<T>() {
                            edit_fn(world, entity, &mut comp);
                            world.entity_mut(entity).insert(comp);
                        }
                    }
                }
                false => {
                    let comp = insert_fn(world, entity);
                    world.entity_mut(handle_id.entity).insert(comp);
                }
            }
        });

        self.get_handle(handle_id)
    }
}

// *****************************************************************************************
// Systems
// *****************************************************************************************
/* fn write_used_components(mut commands: Commands, server: Res<RefComponentServer>) {
    let mut insert_queue = server.insert_queue.write();
    for handle_id in insert_queue.iter() {
        if let Some(spawner) = server.comp_spawner.get(&handle_id.type_id) {
            (spawner.spawn)(&mut commands, handle_id.entity);
        }
    }

    insert_queue.clear();
}

fn free_unused_components(
    mut commands: Commands,
    server: Res<RefComponentServer>,
    valid_query: Query<Entity>,
) {
    let mut potential_frees = server.mark_unused_assets.lock();
    if !potential_frees.is_empty() {
        for potential_free in potential_frees.drain(..) {
            if let Some(spawner) = server.comp_spawner.get(&potential_free.type_id) {
                if valid_query.get(potential_free.entity).is_ok() {
                    (spawner.delete)(&mut commands, potential_free.entity);
                }
            }
        }
    }
}
 */
fn delete_unreferenced_components(
    mut server: ResMut<RefCompServer>,
    valid_query: Query<Entity>,
    mut commands: Commands,
) {
    let ref_changes: Vec<RefChange> = server.channel.receiver.try_iter().collect();
    let ref_counts = &mut server.ref_counts;
    let mut despawn_list: Vec<RefCompHandleId> = Vec::new();
    for ref_change in ref_changes {
        match ref_change {
            RefChange::Increment(handle_id) => *ref_counts.entry(handle_id).or_insert(0) += 1,
            RefChange::Decrement(handle_id) => {
                let entry = ref_counts.entry(handle_id.clone()).or_insert(0);
                *entry -= 1;
                if *entry == 0 {
                    ref_counts.remove(&handle_id);
                    despawn_list.push(handle_id);
                }
            }
        }
    }
    let comp_spawner = &server.comp_spawner;
    for handle_id in despawn_list {
        if let Some(spawner) = comp_spawner.get(&handle_id.type_id) {
            if valid_query.get(handle_id.entity).is_ok() {
                (spawner.delete)(&mut commands, handle_id.entity);
            }
        }
    }
}

// *****************************************************************************************
// App
// *****************************************************************************************
pub trait RefCompExt {
    fn insert_ref_comp_from_world<T: Component + FromWorld>(
        &mut self,
        entity: Entity,
        edit_fn: Option<EditFn<T>>,
    ) -> RefCompHandle<T>;
    fn insert_ref_comp<T: Component>(
        &mut self,
        entity: Entity,
        insert_fn: InsertFn<T>,
        edit_fn: Option<EditFn<T>>,
    ) -> RefCompHandle<T>;
}

impl RefCompExt for World {
    fn insert_ref_comp_from_world<T: Component + FromWorld>(
        &mut self,
        entity: Entity,
        edit_fn: Option<EditFn<T>>,
    ) -> RefCompHandle<T> {
        self.resource_scope(|world, mut ref_comp_server: Mut<RefCompServer>| {
            ref_comp_server.inner_insert_ref_comp_from_world::<T>(world, entity, edit_fn)
        })
    }

    fn insert_ref_comp<T: Component>(
        &mut self,
        entity: Entity,
        insert_fn: InsertFn<T>,
        edit_fn: Option<EditFn<T>>,
    ) -> RefCompHandle<T> {
        self.resource_scope(|world, mut ref_comp_server: Mut<RefCompServer>| {
            ref_comp_server.inner_insert_ref_comp::<T>(world, entity, insert_fn, edit_fn)
        })
    }
}
// *****************************************************************************************
// Structs
// *****************************************************************************************
enum RefCompHandleType {
    Weak,
    Strong(Sender<RefChange>),
}

impl core::fmt::Debug for RefCompHandleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefCompHandleType::Weak => f.write_str("Weak"),
            RefCompHandleType::Strong(_) => f.write_str("Strong"),
        }
    }
}

impl Default for RefCompHandleType {
    fn default() -> Self {
        Self::Weak
    }
}
#[derive(
    Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Reflect, FromReflect,
)]
#[reflect_value(Serialize, Deserialize, PartialEq, Hash)]
pub struct RefCompHandleId {
    pub entity: Entity,
    pub type_id: String,
}

impl RefCompHandleId {
    #[inline]
    pub fn default<T: Component>() -> Self {
        RefCompHandleId {
            entity: Entity::from_raw(u32::MAX),
            type_id: "".to_string(),
        }
    }

    #[inline]
    pub fn new<T: Component>(entity: Entity) -> Self {
        RefCompHandleId {
            entity,
            type_id: type_name::<T>().to_string(),
        }
    }
}

impl<T: Component> RefCompHandle<T> {
    fn strong(id: RefCompHandleId, ref_change_sender: Sender<RefChange>) -> Self {
        ref_change_sender
            .send(RefChange::Increment(id.clone()))
            .unwrap();
        Self {
            id,
            handle_type: RefCompHandleType::Strong(ref_change_sender),
            marker: PhantomData,
        }
    }

    #[inline]
    pub fn weak(id: RefCompHandleId) -> Self {
        Self {
            id,
            handle_type: RefCompHandleType::Weak,
            marker: PhantomData,
        }
    }

    /// Get a copy of this handle as a Weak handle
    pub fn as_weak<U: Component>(&self) -> RefCompHandle<U> {
        RefCompHandle {
            id: self.id.clone(),
            handle_type: RefCompHandleType::Weak,
            marker: PhantomData,
        }
    }

    pub fn is_weak(&self) -> bool {
        matches!(self.handle_type, RefCompHandleType::Weak)
    }

    pub fn is_strong(&self) -> bool {
        matches!(self.handle_type, RefCompHandleType::Strong(_))
    }

    /// Makes this handle Strong if it wasn't already.
    ///
    /// This method requires the corresponding [Assets](crate::Assets) collection
    pub fn make_strong(&mut self, server: &RefCompServer) {
        if self.is_strong() {
            return;
        }
        let sender = server.channel.sender.clone();
        sender.send(RefChange::Increment(self.id.clone())).unwrap();
        self.handle_type = RefCompHandleType::Strong(sender);
    }

    #[inline]
    pub fn clone_weak(&self) -> Self {
        RefCompHandle::weak(self.id.clone())
    }

    pub fn clone_untyped(&self) -> RefCompHandleUntyped {
        match &self.handle_type {
            RefCompHandleType::Strong(sender) => {
                RefCompHandleUntyped::strong(self.id.clone(), sender.clone())
            }
            RefCompHandleType::Weak => RefCompHandleUntyped::weak(self.id.clone()),
        }
    }

    pub fn clone_weak_untyped(&self) -> RefCompHandleUntyped {
        RefCompHandleUntyped::weak(self.id.clone())
    }
}

impl<T: Component> Drop for RefCompHandle<T> {
    fn drop(&mut self) {
        match self.handle_type {
            RefCompHandleType::Strong(ref sender) => {
                // ignore send errors because this means the channel is shut down / the game has
                // stopped
                let _ = sender.send(RefChange::Decrement(self.id.clone()));
            }
            RefCompHandleType::Weak => {}
        }
    }
}

#[derive(Component, Reflect, FromReflect)]
#[reflect(Component)]
pub struct RefCompHandle<T>
where
    T: Component,
{
    /// The ID of the asset as contained within its respective [Assets](crate::Assets) collection
    pub id: RefCompHandleId,
    #[reflect(ignore)]
    handle_type: RefCompHandleType,
    #[reflect(ignore)]
    // NOTE: PhantomData<fn() -> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> T>,
}

impl<T: Component> Default for RefCompHandle<T> {
    fn default() -> Self {
        RefCompHandle::weak(RefCompHandleId::default::<T>())
    }
}

impl<T: Component> std::fmt::Debug for RefCompHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let name = std::any::type_name::<T>().split("::").last().unwrap();
        write!(
            f,
            "{:?}RefCompHandle<{}>({:?})",
            self.handle_type, name, self.id
        )
    }
}

impl<T: Component> Clone for RefCompHandle<T> {
    fn clone(&self) -> Self {
        match self.handle_type {
            RefCompHandleType::Strong(ref sender) => {
                RefCompHandle::strong(self.id.clone(), sender.clone())
            }
            RefCompHandleType::Weak => RefCompHandle::weak(self.id.clone()),
        }
    }
}

#[derive(Debug)]
pub struct RefCompHandleUntyped {
    pub id: RefCompHandleId,
    handle_type: RefCompHandleType,
}

impl RefCompHandleUntyped {
    pub fn weak_from_entity<T: Component>(entity: Entity) -> Self {
        Self {
            id: RefCompHandleId::new::<T>(entity),
            handle_type: RefCompHandleType::Weak,
        }
    }

    fn strong(id: RefCompHandleId, ref_change_sender: Sender<RefChange>) -> Self {
        ref_change_sender
            .send(RefChange::Increment(id.clone()))
            .unwrap();
        Self {
            id,
            handle_type: RefCompHandleType::Strong(ref_change_sender),
        }
    }

    pub fn weak(id: RefCompHandleId) -> Self {
        Self {
            id,
            handle_type: RefCompHandleType::Weak,
        }
    }

    pub fn clone_weak(&self) -> RefCompHandleUntyped {
        RefCompHandleUntyped::weak(self.id.clone())
    }

    pub fn is_weak(&self) -> bool {
        matches!(self.handle_type, RefCompHandleType::Weak)
    }

    pub fn is_strong(&self) -> bool {
        matches!(self.handle_type, RefCompHandleType::Strong(_))
    }

    /// Convert this handle into a typed [Handle].
    ///
    /// The new handle will maintain the Strong or Weak status of the current handle.
    pub fn typed<T: Component>(mut self) -> RefCompHandle<T> {
        let handle_type = match &self.handle_type {
            RefCompHandleType::Strong(sender) => RefCompHandleType::Strong(sender.clone()),
            RefCompHandleType::Weak => RefCompHandleType::Weak,
        };
        // ensure we don't send the RefChange event when "self" is dropped
        self.handle_type = RefCompHandleType::Weak;
        RefCompHandle {
            handle_type,
            id: self.id.clone(),
            marker: PhantomData::default(),
        }
    }
}

impl Drop for RefCompHandleUntyped {
    fn drop(&mut self) {
        match self.handle_type {
            RefCompHandleType::Strong(ref sender) => {
                // ignore send errors because this means the channel is shut down / the game has
                // stopped
                let _ = sender.send(RefChange::Decrement(self.id.clone()));
            }
            RefCompHandleType::Weak => {}
        }
    }
}

enum RefChange {
    Increment(RefCompHandleId),
    Decrement(RefCompHandleId),
}

#[derive(Clone)]
struct RefChangeChannel {
    sender: Sender<RefChange>,
    receiver: Receiver<RefChange>,
}

impl Default for RefChangeChannel {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        RefChangeChannel { sender, receiver }
    }
}

struct RefComponentSpawner {
    delete: fn(&mut Commands, Entity),
}
// *****************************************************************************************
// Functions
// *****************************************************************************************
fn delete_component<T: Component>(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).remove::<T>();
}
