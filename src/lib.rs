use std::{any::type_name, marker::PhantomData, sync::Arc};

use bevy::{
    ecs::{reflect::ReflectComponent, system::Command},
    prelude::{App, Commands, Component, Entity, FromWorld, Plugin, Res, World},
    reflect::{FromReflect, Reflect, ReflectDeserialize},
    utils::{HashMap, HashSet},
};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};

pub struct RefComponentPlugin;

impl Plugin for RefComponentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RefComponentServer>()
            .add_system(write_used_components)
            .add_system(free_unused_components)
            .add_system(mark_unused_assets);
    }
}
// *****************************************************************************************
// Resources
// *****************************************************************************************
#[derive(Default)]
pub struct RefComponentServer {
    channel: Arc<RefChangeChannel>,
    ref_counts: Arc<RwLock<HashMap<RefCompHandleId, usize>>>,
    mark_unused_assets: Arc<Mutex<Vec<RefCompHandleId>>>,
    comp_spawner: HashMap<String, RefComponentSpawner>,
    insert_queue: Arc<RwLock<HashSet<RefCompHandleId>>>,
}

impl RefComponentServer {
    pub fn get_handle<T: Component + FromWorld, I: Into<RefCompHandleId>>(
        &self,
        id: I,
    ) -> RefCompHandle<T> {
        let sender = self.channel.sender.clone();
        RefCompHandle::strong(id.into(), sender)
    }

    pub fn get_handle_untyped<I: Into<RefCompHandleId>>(&self, id: I) -> RefCompHandleUntyped {
        let sender = self.channel.sender.clone();
        RefCompHandleUntyped::strong(id.into(), sender)
    }

    pub fn add_ref_comp<T: Component + FromWorld>(&self, entity: Entity) -> RefCompHandle<T> {
        let ref_counts = self.ref_counts.read();
        let handle_id = RefCompHandleId::new::<T>(entity);

        if !ref_counts.contains_key(&handle_id) {
            let mut insert_queue = self.insert_queue.write();
            insert_queue.insert(handle_id.clone());
        }
        self.get_handle(handle_id)
    }
}

// *****************************************************************************************
// Systems
// *****************************************************************************************
fn write_used_components(mut commands: Commands, server: Res<RefComponentServer>) {
    let mut delete_queue = server.insert_queue.write();
    for handle_id in delete_queue.iter() {
        if let Some(spawner) = server.comp_spawner.get(&handle_id.type_id) {
            (spawner.spawn)(&mut commands, handle_id.entity);
        }
    }

    delete_queue.clear();
}

fn free_unused_components(mut commands: Commands, server: Res<RefComponentServer>) {
    let mut potential_frees = server.mark_unused_assets.lock();
    if !potential_frees.is_empty() {
        let ref_counts = server.ref_counts.read();
        for potential_free in potential_frees.drain(..) {
            if let Some(&0) = ref_counts.get(&potential_free) {
                if let Some(spawner) = server.comp_spawner.get(&potential_free.type_id) {
                    (spawner.delete)(&mut commands, potential_free.entity);
                }
            }
        }
    }
}

fn mark_unused_assets(server: Res<RefComponentServer>) {
    let receiver = &server.channel.receiver;
    let mut ref_counts = server.ref_counts.write();
    let mut potential_frees = None;
    loop {
        let ref_change = match receiver.try_recv() {
            Ok(ref_change) => ref_change,
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => panic!("RefChange channel disconnected."),
        };
        match ref_change {
            RefChange::Increment(handle_id) => *ref_counts.entry(handle_id).or_insert(0) += 1,
            RefChange::Decrement(handle_id) => {
                let entry = ref_counts.entry(handle_id.clone()).or_insert(0);
                *entry -= 1;
                if *entry == 0 {
                    potential_frees
                        .get_or_insert_with(|| server.mark_unused_assets.lock())
                        .push(handle_id);
                }
            }
        }
    }
}

// *****************************************************************************************
// App
// *****************************************************************************************
pub trait AddRefComponentExt {
    fn add_ref_component_type<T: Component + FromWorld>(&mut self) -> &mut Self;
}

impl AddRefComponentExt for App {
    fn add_ref_component_type<T: Component + FromWorld>(&mut self) -> &mut Self {
        self.world.add_ref_component_type::<T>();
        self
    }
}

impl AddRefComponentExt for World {
    fn add_ref_component_type<T: Component + FromWorld>(&mut self) -> &mut Self {
        let mut ref_comp_server = self.get_resource_mut::<RefComponentServer>().unwrap();
        ref_comp_server.comp_spawner.insert(
            type_name::<T>().to_string(),
            RefComponentSpawner {
                spawn: spawn_component::<T>,
                delete: delete_component::<T>,
            },
        );
        self
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
    pub fn default<T: Component + FromWorld>() -> Self {
        RefCompHandleId {
            entity: Entity::from_raw(u32::MAX),
            type_id: "".to_string(),
        }
    }

    #[inline]
    pub fn new<T: Component + FromWorld>(entity: Entity) -> Self {
        RefCompHandleId {
            entity,
            type_id: type_name::<T>().to_string(),
        }
    }
}

impl<T: Component + FromWorld> RefCompHandle<T> {
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
    pub fn as_weak<U: Component + FromWorld>(&self) -> RefCompHandle<U> {
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
    pub fn make_strong(&mut self, server: &RefComponentServer) {
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

impl<T: Component + FromWorld> Drop for RefCompHandle<T> {
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
    T: Component + FromWorld,
{
    /// The ID of the asset as contained within its respective [Assets](crate::Assets) collection
    pub id: RefCompHandleId,
    #[reflect(ignore)]
    handle_type: RefCompHandleType,
    #[reflect(ignore)]
    // NOTE: PhantomData<fn() -> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> T>,
}

impl<T: Component + FromWorld> Default for RefCompHandle<T> {
    fn default() -> Self {
        RefCompHandle::weak(RefCompHandleId::default::<T>())
    }
}

impl<T: Component + FromWorld> std::fmt::Debug for RefCompHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let name = std::any::type_name::<T>().split("::").last().unwrap();
        write!(
            f,
            "{:?}RefCompHandle<{}>({:?})",
            self.handle_type, name, self.id
        )
    }
}

impl<T: Component + FromWorld> Clone for RefCompHandle<T> {
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
    pub fn weak_from_entity<T: Component + FromWorld>(entity: Entity) -> Self {
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
    pub fn typed<T: Component + FromWorld>(mut self) -> RefCompHandle<T> {
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
    spawn: fn(&mut Commands, Entity),
    delete: fn(&mut Commands, Entity),
}
// *****************************************************************************************
// Functions
// *****************************************************************************************
fn spawn_component<T: Component + FromWorld>(commands: &mut Commands, entity: Entity) {
    commands.add(SpawnComponentCommand::<T> {
        entity,
        phantom_data: PhantomData,
    });
}

fn delete_component<T: Component + FromWorld>(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).remove::<T>();
}

struct SpawnComponentCommand<T: Component + FromWorld> {
    entity: Entity,
    phantom_data: PhantomData<T>,
}

impl<T: Component + FromWorld> Command for SpawnComponentCommand<T> {
    fn write(self, world: &mut World) {
        let comp = T::from_world(world);
        world.entity_mut(self.entity).insert(comp);
    }
}
