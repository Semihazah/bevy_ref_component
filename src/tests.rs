use bevy::prelude::*;

use crate::{RefCompBuilder, RefCompExt, RefCompHandle, RefCompPlugin, RefCompServer};

/// Tests if the RefCompServer will insert components that do not currently exist,
/// and if it will remove components that are no longer referenced
#[test]
fn test_insert() {
    let mut app = App::new();

    app.add_plugin(RefCompPlugin).add_startup_system(
        |mut commands: Commands, mut ref_comp_server: ResMut<RefCompServer>| {
            let foo_ent = commands.spawn_empty().id();
            commands.insert_resource(EntityRef(foo_ent));

            let handle = ref_comp_server.insert_ref_comp_fw::<Foo>(&mut commands, foo_ent, None);
            commands.insert_resource(FooHandleRes1(handle));
        },
    );

    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<EntityRef>();
    assert!(world.entity(foo_ent.0).contains::<Foo>());

    world.remove_resource::<FooHandleRes1>();
    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<EntityRef>();
    assert!(!world.entity(foo_ent.0).contains::<Foo>())
}

///Tests if the RefCompServer will not delete components if they are still referenced by a handle.
#[test]
fn test_multi_remove() {
    let mut app = App::new();

    app.add_plugin(RefCompPlugin).add_startup_system(
        |mut commands: Commands, mut ref_comp_server: ResMut<RefCompServer>| {
            let foo_ent = commands.spawn_empty().id();
            commands.insert_resource(EntityRef(foo_ent));

            let handle = ref_comp_server.insert_ref_comp_fw::<Foo>(&mut commands, foo_ent, None);
            commands.insert_resource(FooHandleRes1(handle.clone()));
            commands.insert_resource(FooHandleRes2(handle));
        },
    );

    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<EntityRef>();
    assert!(world.entity(foo_ent.0).contains::<Foo>());

    world.remove_resource::<FooHandleRes1>();
    app.update();

    let world = &mut app.world;
    let foo_ent = world.resource::<EntityRef>();
    assert!(world.entity(foo_ent.0).contains::<Foo>())
}

/// Tests to see if we can add a function to return a new copy of the component
/// to insert for the RefCompServer.
#[test]
fn test_insert_function() {
    let mut app = App::new();

    app.add_plugin(RefCompPlugin).add_startup_system(
        |mut commands: Commands, mut ref_comp_server: ResMut<RefCompServer>| {
            let entity = commands.spawn_empty().id();
            commands.insert_resource(EntityRef(entity));

            let handle = ref_comp_server.insert_ref_comp::<Bar>(
                &mut commands,
                entity,
                |_world, _entity| Bar {
                    string: "I am a test string!".to_string(),
                    integer: 42,
                },
                None,
            );
            commands.insert_resource(BarHandleRes1(handle));
        },
    );

    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>();
    let bar = world.entity(bar_ent.0).get::<Bar>().unwrap();
    assert!(bar.integer == 42);
    assert!(bar.string == "I am a test string!");

    world.remove_resource::<BarHandleRes1>();
    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>();
    assert!(!world.entity(bar_ent.0).contains::<Bar>())
}

/// Tests to see if the insert fn will run if there is already a component present.
/// Note: It should not.
#[test]
fn test_insert_function_overwrite() {
    let mut app = App::new();

    app.add_plugin(RefCompPlugin).add_startup_system(
        |mut commands: Commands, mut ref_comp_server: ResMut<RefCompServer>| {
            let entity = commands.spawn_empty().id();
            commands.insert_resource(EntityRef(entity));

            let handle = ref_comp_server.insert_ref_comp::<Bar>(
                &mut commands,
                entity,
                |_world, _entity| Bar {
                    string: "I am a test string!".to_string(),
                    integer: 42,
                },
                None,
            );
            commands.insert_resource(BarHandleRes1(handle.clone()));
        },
    );

    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>().0;
    let handle = world.insert_ref_comp::<Bar>(
        bar_ent,
        |_world, _entity| Bar {
            string: "I have been changed!".to_string(),
            integer: 69,
        },
        None,
    );

    world.insert_resource(BarHandleRes2(handle));

    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>();
    let bar = world.entity(bar_ent.0).get::<Bar>().unwrap();
    assert!(bar.integer == 42);
    assert!(bar.string == "I am a test string!");

    world.remove_resource::<BarHandleRes1>();

    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>();
    assert!(world.entity(bar_ent.0).contains::<Bar>())
}

/// Tests to see if we can add an edit function to the server that runs if
/// there is already a component present.
#[test]
fn test_insert_edit_function() {
    let mut app = App::new();

    app.add_plugin(RefCompPlugin).add_startup_system(
        |mut commands: Commands, mut ref_comp_server: ResMut<RefCompServer>| {
            let entity = commands.spawn_empty().id();
            commands.insert_resource(EntityRef(entity));

            let handle = ref_comp_server.insert_ref_comp::<Bar>(
                &mut commands,
                entity,
                |_world, _entity| Bar {
                    string: "I am a test string!".to_string(),
                    integer: 42,
                },
                None,
            );
            commands.insert_resource(BarHandleRes2(handle.clone()));
        },
    );

    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>().0;
    let handle = world.insert_ref_comp::<Bar>(
        bar_ent,
        |_world, _entity| Bar {
            string: "I have been changed!".to_string(),
            integer: 69,
        },
        Some(|_world, _entity, bar| {
            bar.string = "I have been changed! For good.".to_string();
            bar.integer = 12;
        }),
    );

    world.insert_resource(BarHandleRes1(handle));

    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>();
    let bar = world.entity(bar_ent.0).get::<Bar>().unwrap();
    assert!(bar.integer == 12);
    assert!(bar.string == "I have been changed! For good.");
}

#[test]
fn test_builder() {
    let mut app = App::new();

    app.add_plugin(RefCompPlugin).add_startup_system(
        |mut commands: Commands, mut ref_comp_server: ResMut<RefCompServer>| {
            let entity = commands.spawn_empty().id();
            commands.insert_resource(EntityRef(entity));

            let handle = RefCompBuilder::new(entity, |_world, _entity| Bar {
                string: "I am a test string!".to_string(),
                integer: 42,
            })
            .build(&mut commands, &mut ref_comp_server);
            /*             let handle = ref_comp_server.add_ref_comp::<Bar>(
                &mut commands,
                entity,
                |_world, _entity| Bar {
                    string: "I am a test string!".to_string(),
                    integer: 42,
                },
                None,
            ); */
            commands.insert_resource(BarHandleRes1(handle.clone()));
        },
    );

    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>();
    let bar = world.entity(bar_ent.0).get::<Bar>().unwrap();
    assert!(bar.integer == 42);
    assert!(bar.string == "I am a test string!");

    world.remove_resource::<BarHandleRes1>();
    app.update();

    let world = &mut app.world;
    let bar_ent = world.resource::<EntityRef>();
    assert!(!world.entity(bar_ent.0).contains::<Bar>())
}

#[derive(Component, Default)]
struct Foo;

#[derive(Resource)]
struct EntityRef(Entity);

#[derive(Resource)]
struct FooHandleRes1(RefCompHandle<Foo>);

#[derive(Resource)]
struct FooHandleRes2(RefCompHandle<Foo>);

#[derive(Component, Default)]
struct Bar {
    string: String,
    integer: u32,
}

#[derive(Resource)]
struct BarHandleRes1(RefCompHandle<Bar>);

#[derive(Resource)]
struct BarHandleRes2(RefCompHandle<Bar>);