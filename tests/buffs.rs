use bevy_app::App;
use bevy_ecs::{component::Component, system::{RunSystemOnce, Commands, Query}, entity::Entity, query::With};
use bevy_hierarchy::BuildChildren;
use bevy_salo::{SaveLoadPlugin, methods::{Ron, Postcard, SerdeJson}, Marker, PathName, SaveLoadExtension, All};
use std::borrow::Cow;

macro_rules! component {
    ($($name: ident),*) => {
        
        $(#[derive(Debug, Clone, Copy, Component, Default)]
        struct $name;)*
    };
}

macro_rules! salo {
    ($($name: ident $body: tt $($self: ident => $expr: expr)?),*) => {
        
        $(#[derive(Debug, Clone, Component, Default, serde::Serialize, serde::Deserialize)]
        struct $name $body

        #[allow(unused)]
        impl bevy_salo::SaveLoadCore for $name{
            fn type_name() -> Cow<'static, str> {
                Cow::Borrowed(stringify!($name))
            }
            $(
                fn path_name(&self) -> Option<Cow<'static, str>> {
                    let $self = self;
                    Some($expr.clone().into())
                }
            )?
        })*
    };
}

component!(Units, Players, Enemies);
salo!(
    Unit {
        name: String,
        hp: i32,
    } this => this.name, 
    Weapon {} this => "mainhand", 
    Offhand {} this => "offhand", 
    Item {
        // note: not unique
        name: String
    }, 
    Buff {
        stat: String,
        value: f32
    }
);

#[derive(Debug, Clone, Component, serde::Serialize, serde::Deserialize)]
pub struct BuffPtr(Entity);


#[test]
pub fn test_cases () {
    test::<All<SerdeJson>>(None);
    test::<All<Ron>>(Some(".ron"));
    test::<All<Postcard>>(None);
}

pub fn test<P: Marker>(ext: Option<&str>) {
    let mut app = App::new();
    app.add_plugins(SaveLoadPlugin::new::<P>()
        .register::<Unit>()
        .register::<Weapon>()
        .register::<Offhand>()
        .register::<Buff>()
        .register::<Item>()
    );

    app.world.run_system_once(|mut commands: Commands| {
        commands.spawn(Units).with_children(|builder| {
            builder.spawn((Players, PathName::new("Players"))).with_children(|builder| {
                builder.spawn(Unit {
                    name: "John".to_owned(),
                    hp: 32,
                }).with_children(|b| {
                    b.spawn((
                        Weapon {},
                        Item {
                            name: "Rapier".to_owned()
                        },
                    )).with_children(|b| {
                        b.spawn(Buff {
                            stat: "Damage".to_owned(),
                            value: 12.5,
                        });
                        b.spawn(Buff {
                            stat: "Speed".to_owned(),
                            value: 4.0,
                        });
                    });
                    b.spawn((
                        Offhand {},
                        Item {
                            name: "Buckler".to_owned()
                        },
                    )).with_children(|b| {
                        b.spawn(Buff {
                            stat: "Defense".to_owned(),
                            value: 6.5,
                        });
                    });
                    b.spawn((
                        Item {
                            name: "HP Ring".to_owned()
                        },
                    )).with_children(|b| {
                        b.spawn(Buff {
                            stat: "Hp".to_owned(),
                            value: 10.0,
                        });
                    });
                    b.spawn(Item {
                        name: "HP Potion".to_owned()
                    });
                    b.spawn(Item {
                        name: "HP Potion".to_owned()
                    });
                });
                builder.spawn(Unit {
                    name: "Jane".to_owned(),
                    hp: 28,
                }).with_children(|b| {
                    b.spawn((
                        Weapon {},
                        Item {
                            name: "Wooden Staff".to_owned()
                        },
                    )).with_children(|b| {
                        b.spawn(Buff {
                            stat: "Magic".to_owned(),
                            value: 6.5,
                        });
                    });
                    b.spawn((
                        Item {
                            name: "Fire Ring".to_owned()
                        },
                    )).with_children(|b| {
                        b.spawn(Buff {
                            stat: "Fire Damage".to_owned(),
                            value: 5.0,
                        });
                    });
                    b.spawn(Item {
                        name: "Herb".to_owned()
                    });
                    b.spawn(Item {
                        name: "Mana Potion".to_owned()
                    });
                });
            });
            builder.spawn((Enemies, PathName::new("Enemies")));
        });
    });
    let buffer = app.world.save_to::<P, Vec<u8>>().unwrap();
    app.world.remove_serialized_components::<P>();
    assert_eq!(app.world.run_system_once(|e: Query<&Units>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Players>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Enemies>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Unit>| e.iter().count()), 0);
    assert_eq!(app.world.run_system_once(|e: Query<&Weapon>| e.iter().count()), 0);
    assert_eq!(app.world.run_system_once(|e: Query<&Offhand>| e.iter().count()), 0);
    assert_eq!(app.world.run_system_once(|e: Query<&Item>| e.iter().count()), 0);
    assert_eq!(app.world.run_system_once(|e: Query<&Buff>| e.iter().count()), 0);

    app.world.load_from_bytes::<P>(&buffer);
    assert_eq!(app.world.run_system_once(|e: Query<&Units>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Players>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Enemies>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Unit>| e.iter().count()), 2);
    assert_eq!(app.world.run_system_once(|e: Query<&Weapon>| e.iter().count()), 2);
    assert_eq!(app.world.run_system_once(|e: Query<&Offhand>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Item>| e.iter().count()), 9);
    assert_eq!(app.world.run_system_once(|e: Query<&Buff>| e.iter().count()), 6);


    // Load again, note Unit, Weapon and Offhand are named, they should not duplicate and just update.
    app.world.load_from_bytes::<P>(&buffer);
    assert_eq!(app.world.run_system_once(|e: Query<&Units>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Players>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Enemies>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Unit>| e.iter().count()), 2);
    assert_eq!(app.world.run_system_once(|e: Query<&Weapon>| e.iter().count()), 2);
    assert_eq!(app.world.run_system_once(|e: Query<&Offhand>| e.iter().count()), 1);
    // 3 of these are associated with Weapon or Offhand.
    assert_eq!(app.world.run_system_once(|e: Query<&Item>| e.iter().count()), (9 - 3) * 2 + 3);
    assert_eq!(app.world.run_system_once(|e: Query<&Buff>| e.iter().count()), 6 * 2);
    // swap enemy and player
    app.world.run_system_once(|mut q: Query<&mut PathName, With<Players>>| {
        q.single_mut().set_static("OriginalPlayers")
    });
    app.world.run_system_once(|mut q: Query<&mut PathName, With<Enemies>>| {
        q.single_mut().set_static("Players")
    });
    // This doubles the amount of units due to new paths
    app.world.load_from_bytes::<P>(&buffer);
    assert_eq!(app.world.run_system_once(|e: Query<&Units>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Players>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Enemies>| e.iter().count()), 1);
    assert_eq!(app.world.run_system_once(|e: Query<&Unit>| e.iter().count()), 4);
    assert_eq!(app.world.run_system_once(|e: Query<&Weapon>| e.iter().count()), 4);
    assert_eq!(app.world.run_system_once(|e: Query<&Offhand>| e.iter().count()), 2);
    assert_eq!(app.world.run_system_once(|e: Query<&Item>| e.iter().count()), 15 + 9);
    assert_eq!(app.world.run_system_once(|e: Query<&Buff>| e.iter().count()), 6 * 2 + 6);
    
    if let Some(ext) = ext{
        app.world.save_to_file::<P>(&format!("test_buffs{}", ext));
    }
}