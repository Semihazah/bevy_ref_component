use bevy::prelude::*;

use crate::{RefCompHandle, RefComponentPlugin, RefComponentServer, RefCompExt};

#[test]
fn test_insert() {
    let mut app = App::new();

    app.add_plugin(RefComponentPlugin)
        .add_startup_system(
            |mut commands: Commands, mut ref_comp_server: ResMut<RefComponentServer>| {
                let foo_ent = commands.spawn().id();
                commands.insert_resource(EntityRef(foo_ent));

                
                let handle = ref_comp_server.add_ref_comp_from_world::<Foo>(&mut commands, foo_ent);
                commands.insert_resource(handle);
            },
        );

    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<EntityRef>();
    assert!(world.entity(foo_ent.0).contains::<Foo>());

    world.remove_resource::<RefCompHandle<Foo>>();
    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<EntityRef>();
    assert!(!world.entity(foo_ent.0).contains::<Foo>())
}

#[test]
fn test_multi_remove() {
    let mut app = App::new();

    app.add_plugin(RefComponentPlugin)
        .add_startup_system(
            |mut commands: Commands, mut ref_comp_server: ResMut<RefComponentServer>| {
                let foo_ent = commands.spawn().id();
                commands.insert_resource(EntityRef(foo_ent));

                
                let handle = ref_comp_server.add_ref_comp_from_world::<Foo>(&mut commands, foo_ent);
                commands.insert_resource(handle.clone());
                commands.insert_resource(FooHandleRes(handle));
            },
        );

    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<EntityRef>();
    assert!(world.entity(foo_ent.0).contains::<Foo>());

    world.remove_resource::<RefCompHandle<Foo>>();
    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<EntityRef>();
    assert!(world.entity(foo_ent.0).contains::<Foo>())
}

#[test]
fn test_insert_function() {
    let mut app = App::new();

    app.add_plugin(RefComponentPlugin)
        .add_startup_system(
            |mut commands: Commands, mut ref_comp_server: ResMut<RefComponentServer>| {
                let entity = commands.spawn().id();
                commands.insert_resource(EntityRef(entity));

                
                let handle = ref_comp_server.add_ref_comp::<Bar>(&mut commands, entity, |commands, entity| {
                    commands.entity(entity).insert(Bar {
                        string: "I am a test string!".to_string(),
                        integer: 42,
                    });
                });
                commands.insert_resource(handle.clone());
            },
        );

    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>();
    let bar = world.entity(bar_ent.0).get::<Bar>().unwrap();
    assert!(bar.integer == 42);
    assert!(bar.string == "I am a test string!");

    world.remove_resource::<RefCompHandle<Bar>>();
    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>();
    assert!(!world.entity(bar_ent.0).contains::<Bar>())
}

#[test]
fn test_insert_function_overwrite() {
    let mut app = App::new();

    app.add_plugin(RefComponentPlugin)
        .add_startup_system(
            |mut commands: Commands, mut ref_comp_server: ResMut<RefComponentServer>| {
                let entity = commands.spawn().id();
                commands.insert_resource(EntityRef(entity));

                
                let handle = ref_comp_server.add_ref_comp::<Bar>(&mut commands, entity, |commands, entity| {
                    commands.entity(entity).insert(Bar {
                        string: "I am a test string!".to_string(),
                        integer: 42,
                    });
                });
                commands.insert_resource(handle.clone());
            },
        );

    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>().0;
    let handle = world.insert_ref_comp::<Bar>(bar_ent, |world, entity| {
        world.entity_mut(entity).insert(Bar {
            string: "I have been changed!".to_string(),
            integer: 69,
        });
    });

    world.insert_resource(BarHandleRes(handle));

    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>();
    let bar = world.entity(bar_ent.0).get::<Bar>().unwrap();
    assert!(bar.integer == 42);
    assert!(bar.string == "I am a test string!");

    world.remove_resource::<RefCompHandle<Bar>>();

    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>();
    assert!(world.entity(bar_ent.0).contains::<Bar>())
}

#[derive(Component, Default)]
struct Foo;

struct EntityRef(Entity);

struct FooHandleRes(RefCompHandle<Foo>);

#[derive(Component, Default)]
struct Bar {
    string: String,
    integer: u32,
}

struct BarHandleRes(RefCompHandle<Bar>);