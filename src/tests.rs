use bevy::prelude::*;

use crate::{AddRefComponentExt, RefCompHandle, RefComponentPlugin, RefComponentServer};

#[test]
fn test_insert() {
    let mut app = App::new();

    app.add_plugin(RefComponentPlugin)
        .add_ref_component_type::<Foo>()
        .add_startup_system(
            |mut commands: Commands, ref_comp_server: Res<RefComponentServer>| {
                let foo_ent = commands.spawn().id();
                commands.insert_resource(FooRef(foo_ent));

                let handle = ref_comp_server.add_ref_comp::<Foo>(foo_ent);
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

#[derive(Component, Default)]
struct Foo;

struct FooRef(Entity);
