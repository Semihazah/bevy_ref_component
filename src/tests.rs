use bevy::prelude::*;

use crate::{RefCompHandle, RefComponentPlugin, RefComponentServer};

#[test]
fn test_insert() {
    let mut app = App::new();

    app.add_plugin(RefComponentPlugin)
        .add_startup_system(
            |mut commands: Commands, mut ref_comp_server: ResMut<RefComponentServer>| {
                let foo_ent = commands.spawn().id();
                commands.insert_resource(FooRef(foo_ent));

                
                let handle = ref_comp_server.add_ref_comp::<Foo>(&mut commands, foo_ent);
                commands.insert_resource(handle);
            },
        );

    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<FooRef>();
    assert!(world.entity(foo_ent.0).contains::<Foo>());

    world.remove_resource::<RefCompHandle<Foo>>();
    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<FooRef>();
    assert!(!world.entity(foo_ent.0).contains::<Foo>())
}

#[test]
fn test_multi_remove() {
    let mut app = App::new();

    app.add_plugin(RefComponentPlugin)
        .add_startup_system(
            |mut commands: Commands, mut ref_comp_server: ResMut<RefComponentServer>| {
                let foo_ent = commands.spawn().id();
                commands.insert_resource(FooRef(foo_ent));

                
                let handle = ref_comp_server.add_ref_comp::<Foo>(&mut commands, foo_ent);
                commands.insert_resource(handle.clone());
                commands.insert_resource(HandleRes(handle));
            },
        );

    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<FooRef>();
    assert!(world.entity(foo_ent.0).contains::<Foo>());

    world.remove_resource::<RefCompHandle<Foo>>();
    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<FooRef>();
    assert!(world.entity(foo_ent.0).contains::<Foo>())
}

#[derive(Component, Default)]
struct Foo;

struct FooRef(Entity);

struct HandleRes(RefCompHandle<Foo>);