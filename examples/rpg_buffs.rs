use bevy_app::{App, Update, Startup};
use bevy_ecs::{system::{Commands, Res, ResMut}, component::Component, world::World};
use bevy_hierarchy::BuildChildren;
use bevy_salo::{SaveLoadPlugin, SaveLoadCore, SaveLoadExtension, methods::Ron, interned_enum, SaveLoad, EntityPath};
use serde::{Serialize, Deserialize};

#[derive(Debug, Component, Clone, Serialize, Deserialize, Default)]
struct Human {
    name: String,
    age: i32,
    hp: f32,
    attack: f32,
}

impl SaveLoadCore for Human {
    fn type_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("human")
    }

    fn path_name(&self) -> Option<std::borrow::Cow<'static, str>> {
        Some(self.name.clone().into())
    }
}

interned_enum!(StatServer, Stat: u64 {
    Attack,
    Defense,
    Magic,
});

#[derive(Debug, Component, Clone)]
struct Buff {
    name: String,
    stat: Stat,
    value: f32,
}


#[derive(Debug, Component, Clone, Serialize, Deserialize, Default)]
struct BuffSerde {
    name: String,
    stat: String,
    value: f32,
}

impl SaveLoad for Buff {
    type Ser<'ser> = BuffSerde;
    type De = BuffSerde;

    type Context<'w, 's> = Res<'w, StatServer>;
    type ContextMut<'w, 's> = ResMut<'s, StatServer>;

    fn type_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("buff")
    }

    fn to_serializable<'t, 'w, 's>(&'t self, 
        _: bevy_ecs::entity::Entity,
        _: impl Fn(bevy_ecs::entity::Entity) -> EntityPath,
        ctx: &'t bevy_ecs::system::SystemParamItem<Self::Context<'w, 's>>
    ) -> Self::Ser<'t> {
        BuffSerde {
            name: self.name.clone(),
            stat: ctx.as_str(self.stat).to_owned(),
            value: self.value.to_owned(),
        }
    }

    fn from_deserialize<'w, 's>(
        de: Self::De, 
        _: &mut Commands,
        _: bevy_ecs::entity::Entity,
        _: impl FnMut(&mut Commands, &EntityPath) -> bevy_ecs::entity::Entity, 
        ctx: &mut bevy_ecs::system::SystemParamItem<Self::ContextMut<'w, 's>>
    ) -> Self {
        Self { 
            name: de.name, 
            stat: ctx.get(&de.stat), 
            value: de.value.to_owned(), 
        }
    }
}

type All = bevy_salo::All<Ron<true>>;

pub fn main() {
    App::new()
        .add_plugins(SaveLoadPlugin::new::<All>()
            .register::<Human>()
            .register::<Buff>()
        )
        .add_systems(Startup, spawn)
        .add_systems(Update, serialize)
        .init_resource::<StatServer>()
        .run()
    ;
}

pub fn spawn(mut commands: Commands) {
    let children = [
        commands.spawn(Buff {
            name: "Attack Up!".to_owned(),
            stat: Stat::Attack,
            value: 4.0,
        }).id(),
        commands.spawn(Buff {
            name: "Defense Up!".to_owned(),
            stat: Stat::Defense,
            value: 6.7,
        }).id(),
    ];
    commands.spawn(Human {
        name: "Jimmy".to_owned(),
        age: 32,
        hp: 42.1,
        attack: 7.8,
    }).push_children(&children);
    let children = [
        commands.spawn(Buff {
            name: "Magic Up!".to_owned(),
            stat: Stat::Magic,
            value: 4.0,
        }).id(),
    ];
    commands.spawn(Human {
        name: "Sammy".to_owned(),
        age: 27,
        hp: 55.6,
        attack: 2.4,
    }).push_children(&children);
}

pub fn serialize(world: &mut World) {
    world.save_to_file::<All>("test.ron");
    world.load_from_file::<All>("test.ron");
    world.save_to_file::<All>("duplicated.ron");
    world.remove_serialized_components::<All>();
    world.save_to_file::<All>("cleared.ron");
    world.load_from_file::<All>("test.ron");
    world.save_to_file::<All>("roundtrip.ron");

}