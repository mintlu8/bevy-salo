//! bevy_salo (SAveLOad) is an ECS based serialization crate for bevy_ecs.
//! 
//! # Unique Features
//! 
//! * Not dependent on reflection or bevy_app.
//! * Greater user control.
//! * Match entities by name.
//! * Custom ser/de methods that can load resources, spawn entities, etc.
//! 
//! # Getting Started
//! 
//! To get started, register the plugin and all the types you need to serialize.
//! ```
//! # use bevy_app::App;
//! # use bevy_salo::{*, methods::SerdeJson};
//! # let mut app = App::new();
//! # macro_rules! comps {
//! #     ($($name: ident),*) => {
//! #         $(#[derive(bevy_ecs::component::Component, serde::Serialize, serde::Deserialize)]
//! #         struct $name;
//! #         impl SaveLoadCore for $name {})*
//! #     };
//! # }
//! # comps!(Unit, Weapon, Stat, Hp);
//! // It is recommanded to alias here.
//! type All = bevy_salo::All<SerdeJson>;
//! app.add_plugins(
//!     SaveLoadPlugin::new::<All>()
//!         .register::<Unit>()
//!         .register::<Weapon>()
//!         .register::<Stat>()
//!         .register::<Hp>()
//! );
//! ```
//! 
//! Generic types (unforunately) need to be registered separately.
//! 
//! ```
//! # /*
//! SaveLoadPlugin::new::<All>()
//!     .register::<Unit<Human>>()
//!     .register::<Unit<Monster>>()
//! );
//! # */
//! ```
//! 
//! `All` serializes all entities, to narrow the scope with a marker component:
//! 
//! ```
//! # use bevy_app::App;
//! # use bevy_ecs::component::Component;
//! # use bevy_salo::{*, methods::SerdeJson};
//! # let mut app = App::new();
//! # #[derive(Component, serde::Serialize, serde::Deserialize)]
//! # struct Unit;
//! # impl SaveLoadCore for Unit {}
//! #[derive(Debug, Default, Component)]
//! pub struct SaLo;
//! 
//! impl bevy_salo::MarkerComponent for SaLo {
//!     // Set the serialization method here.
//!     type Method = SerdeJson;
//! }
//! 
//! app.add_plugins(
//!     SaveLoadPlugin::new::<SaLo>()
//!         .register::<Unit>()
//! );
//! ```
//! 
//! # Usage
//! 
//! `bevy_salo` creates [schedules](bevy_ecs::schedule::Schedule) for
//! serialization and deserialization. If you have access to a `&mut World`,
//! you can use these extension methods. You can either use a system or 
//! implement a custom [`Command`](bevy_ecs::system::Command).
//! 
//! ```
//! # /*
//! world.load_from_file::<All>("test.ron");
//! world.save_to_file::<All>("test.json");
//! world.deserialize_from::<All>(bytes);
//! let bytes = world.serialize_to::<All>();
//! # */
//! ```
//! 
//! Deserialize does not remove existing items.
//! To cleanup, choose one of these functions 
//! that best suit your use case.
//! 
//! ```
//! # /*
//! // remove all serialized components, does not despawn entities
//! world.remove_serialized_components::<All>();
//! 
//! // despawn entities with a marker.
//! world.despawn_with_marker::<Marker>();
//! # */
//! ```
//! 
//! # Traits
//! 
//! For your structs to work with `bevy_salo`, you need to implement one of three traits:
//! [`SaveLoadCore`], [`SaveLoadMapped`] and [`SaveLoad`].
//! 
//! ## SaveLoadCore
//! 
//! [`SaveLoadCore`] can be easily implemented on any struct implementing 
//! `serde::Serialize` and `serde::Deserialize`.
//! 
//! ```
//! # use serde::{Serialize, Deserialize};
//! # use bevy_ecs::component::Component;
//! # use bevy_salo::SaveLoadCore;
//! #[derive(Serialize, Deserialize, Component)]
//! struct Weapon {
//!     name: String,
//!     damage: f32,
//!     cost: i32,
//! }
//! impl SaveLoadCore for Weapon {}
//! ```
//! 
//! However you should almost always overwrite the `type_name` function on `SaveLoadCore`,
//! since the default implementation `Any::type_name()` is unstable across rust verions and 
//! namespace dependent, which could break the save format when refactoring.
//! 
//! ```
//! # use serde::{Serialize, Deserialize};
//! # use bevy_ecs::component::Component;
//! # use bevy_salo::SaveLoadCore;
//! # use std::borrow::Cow;
//! # #[derive(Serialize, Deserialize, Component)]
//! # struct Weapon  { name: String }
//! impl SaveLoadCore for Weapon {
//!     // This has to be unique across all registered types.
//!     fn type_name() -> Cow<'static, str> {
//!         Cow::Borrowed("weapon")
//!     }
//!     // Provide a path name for the associated entity.
//!     fn path_name(&self) -> Option<Cow<'static, str>> {
//!         Some(self.name.clone().into())
//!     }
//! }
//! ```
//! 
//! ## SaveLoadMapped
//! 
//! [`SaveLoadMapped`] is just like `SaveLoadCore` but you can map non-serializable struct into 
//! a serializable.
//! 
//! ## SaveLoad
//! 
//! Implementing [`SaveLoad`] allows you to do arbitrary things during 
//! serialization and deserialization. Checkout its documentation for more information. 
//! 
//! String interning example:
//! 
//! ```rust
//! # use bevy_ecs::system::{Res, ResMut};
//! # use bevy_salo::{EntityPath, interned_enum, SaveLoad};
//! # use bevy_ecs::entity::Entity;
//! # use bevy_ecs::system::Commands;
//! interned_enum!(ElementsServer, Elements: u64 {
//!     Water, Earth, Fire, Air
//! });
//!
//! impl SaveLoad for Elements {
//!     type Ser<'ser> = &'ser str;
//!     type De = String;
//!     type Context<'w, 's> = Res<'w, ElementsServer>;
//!     type ContextMut<'w, 's> = ResMut<'s, ElementsServer>;
//!
//!     fn to_serializable<'t, 'w, 's>(&'t self, 
//!         _: Entity,
//!         _: impl Fn(Entity) -> EntityPath, 
//!         res: &'t Res<'w, ElementsServer>
//!     ) -> Self::Ser<'t> {
//!         res.as_str(*self)
//!     }
//!
//!     fn from_deserialize<'w, 's>(
//!         de: Self::De, 
//!         _: &mut Commands,
//!         _: Entity,
//!         _: impl FnMut(&mut Commands, &EntityPath) -> Entity, 
//!         res: &mut ResMut<'s, ElementsServer>
//!     ) -> Self {
//!         res.get(&de)
//!     }
//! }
//! ```
//! 
//! # Paths
//! 
//! `bevy_salo` records each entity as either its Entity ID or its path. 
//! Entity ID is only used for disambiguation, 
//! while path allow matching with existing entity.
//! 
//! Each component can optionally provide a name with the `path_name` function
//! defined in the aforementioned traits for their associated entity. 
//! The [`PathName`] component can be used instead for non-serialized entities.
//! 
//! In this example
//! the entity has the path name `"John"`.
//! 
//! ```
//! # /*
//! Entity {
//!     Character => Some("John"),
//!     Weapon => None,
//!     Armor => None,
//! }
//! # */
//! ```
//! 
//! 
//! This panics for conflicting names.
//! 
//! ```
//! # /*
//! Entity {
//!     Character => Some("John"),
//!     Role => Some("Protagonist"),
//! }
//! # */
//! ```
//! 
//! An entity's path contains all its named ancestors. Consider this entity:
//! 
//! ```rust
//! # /*
//! (root)::window::(unnamed)::characters::John::weapon
//! # */
//! ```
//! 
//! The weapon's path is `characters::John::weapon`, while everything before its
//! unnamed ancestor is ignored. This is helpful when you want to insert `"John"`
//! into an existing entity `"characters"`.
//! 
//! 
//! Pathed entities must have unique paths, but duplicated names are allowed.
//! 
//! ```
//! # /* 
//! // legal, although both named `weapon`, paths are different
//! characters::John::weapon
//! characters::Jane::weapon
//! 
//! // illegal, 2 entities with path `characters`
//! characters::John::weapon
//! characters::Jane::(unnamed)::characters
//! # */
//! ```
//! 
//! When serializing, non-serializing parents of 
//! serialized children must be named.
//! 
//! ```
//! # /* 
//! // legal, parent is root
//! (root)::[Named]
//! 
//! // legal, parent is named
//! Named::[Named]
//! 
//! // illegal, parent is not named, cannot deserialize correctly
//! (unnamed)::[Named]
//! # */
//! ```
//! 

pub mod methods;
mod saveload;

use bevy_ecs::bundle::Bundle;
use bevy_ecs::query::{ReadOnlyWorldQuery, With};
use bevy_ecs::world::World;
use methods::{SerializationMethod, SerdeJson};
pub use saveload::*;
use schedules::{SaveSchedule, ResetSchedule};
use sealed::SerializationResult;
use std::borrow::Cow;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy_ecs::component::Component;
use bevy_ecs::system::{Resource, RunSystemOnce, Query};

pub(crate) mod sealed;

pub mod schedules;

mod serde_impls;
mod interner;

/// A special marker that represents no need for marker types. 
/// 
/// # Note
/// 
/// If used you should always alias this for correctness and ergonomics,
/// since each generics combinations is a unique schedule.
/// If you use `All<SerdeJson>` instead of `All<SerdeJson<false>>`, 
/// you will be running an unregistered schedule.
/// 
/// ```rust
/// # use bevy_salo::methods::SerdeJson;
/// type All = bevy_salo::All<SerdeJson<false>>;
/// ```
/// 
/// # Fork
/// 
/// Since schedules are unique per marker type, you can "fork" this by supplying a different `FORK` value.
/// 
/// ```rust
/// # use bevy_salo::methods::Postcard;
/// type Schedule2 = bevy_salo::All<Postcard, '2'>;
/// ```
#[derive(Debug)]
pub struct All<S: SerializationMethod=SerdeJson, const FORK: char='\0'>(PhantomData<S>);

/// Implement this on your marker types.
pub trait MarkerComponent: Component + Debug + Default + Send + Sync + 'static {
    type Method: SerializationMethod;
}

/// Provides path names for entities, including non-serialized ones.
#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct PathName(Cow<'static, str>);

impl PathName {

    pub fn new(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }

    pub fn new_owned(s: String) -> Self {
        Self(Cow::Owned(s))
    }

    pub fn get(&self) -> Cow<'static, str> {
        self.0.clone()
    }

    pub fn set(&mut self, s: impl Into<String>) {
        self.0 = Cow::Owned(s.into())
    }

    pub fn set_static(&mut self, s: &'static str) {
        self.0 = Cow::Borrowed(s)
    }
}

/// Plugin for saving and loading.
pub struct SaveLoadPlugin<Marker=All, Children = ()> (PhantomData<(Marker, Children)>);

impl SaveLoadPlugin {
    /// Create a new save load plugin with the given marker.
    pub fn new<M: Marker>() -> SaveLoadPlugin::<M> {
        SaveLoadPlugin(PhantomData)
    }
}

/// A marker component with a serialization method.
pub trait Marker: sealed::MarkerSeal + std::fmt::Debug + Default + Send + Sync + 'static {
    type Method: SerializationMethod;
    type Query: ReadOnlyWorldQuery;
    type Bundle: Bundle + Default;
    const IS_ALL: bool;
}

impl<T> sealed::MarkerSeal for T where T: MarkerComponent {}

impl<T> Marker for T where T: MarkerComponent {
    type Method = T::Method;
    type Query = With<T>;
    type Bundle = T;
    const IS_ALL: bool = false;
}

impl<S: SerializationMethod, const FORK: char> sealed::MarkerSeal for All<S, FORK> {}

impl<S: SerializationMethod, const FORK: char> Marker for All<S, FORK> {
    type Method = S;
    type Query = ();
    type Bundle = ();
    const IS_ALL: bool = true;
}


/// Extension methods for [`World`].
pub trait SaveLoadExtension: sealed::Sealed {
    /// Serialize all data with a marker to a file.
    #[cfg(feature="fs")]
    fn save_to_file<M: Marker>(&mut self, file: &str);
    /// Serialize all data with a marker to a `String` or a `Vec<u8>`.
    fn save_to<M: Marker, S: SerializationResult>(&mut self) -> Option<S>;
    /// Deserialize all data with a marker from a file.
    #[cfg(feature="fs")]
    fn load_from_file<M: Marker>(&mut self, file: &str);
    /// Deserialize all data with a marker from a `&[u8]`.
    fn load_from_bytes<M: Marker>(&mut self, value: &[u8]);
    /// Deserialize all data with a marker from a `String` or a `Vec<u8>`.
    fn load_from<M: Marker, S: SerializationResult>(&mut self, value: &S);
    /// Remove all components marked with `SaveLoad` and marker. Maybe useful when reloading a save.
    /// 
    /// Note this does not remove entities.
    fn remove_serialized_components<M: Marker>(&mut self);
    /// Despawn all entities with a marker.
    ///
    /// `All` cannot be used here and is hardcoded to fail.
    fn despawn_with_marker<M: Marker>(&mut self);
}

impl sealed::Sealed for World {}

impl SaveLoadExtension for World {
    #[cfg(feature="fs")]
    fn save_to_file<M: Marker>(&mut self, file: &str) {
        self.remove_resource::<BytesOutput<M>>();
        self.remove_resource::<StringOutput<M>>();
        self.insert_resource(FileOutput::<M>::new(file));
        self.run_schedule(SaveSchedule::with_marker::<M>())
    }

    fn save_to<M: Marker, S: SerializationResult>(&mut self) -> Option<S> {
        #[cfg(feature="fs")]
        self.remove_resource::<FileOutput<M>>();
        self.remove_resource::<BytesOutput<M>>();
        self.remove_resource::<StringOutput<M>>();
        S::setup::<M>(self);
        self.run_schedule(SaveSchedule::with_marker::<M>());
        S::get::<M>(self)
    }

    #[cfg(feature="fs")]
    fn load_from_file<M: Marker>(&mut self, file: &str) {
        use crate::schedules::LoadSchedule;
        self.remove_resource::<BytesInput<M>>();
        self.insert_resource(FileInput::<M>::new(file));
        self.run_schedule(LoadSchedule::with_marker::<M>());
    }

    fn load_from<M: Marker, S: SerializationResult>(&mut self, value: &S) {
        use crate::schedules::LoadSchedule;
        self.remove_resource::<BytesInput<M>>();
        self.insert_resource(BytesInput::<M>::new(value.as_bytes()));
        self.run_schedule(LoadSchedule::with_marker::<M>());
    }

    fn load_from_bytes<M: Marker>(&mut self, value: &[u8]) {
        use crate::schedules::LoadSchedule;
        self.remove_resource::<BytesInput<M>>();
        self.insert_resource(BytesInput::<M>::new(value));
        self.run_schedule(LoadSchedule::with_marker::<M>());
    }
    
    fn remove_serialized_components<M: Marker>(&mut self) {
        self.run_schedule(ResetSchedule::with_marker::<M>());
    }
    fn despawn_with_marker<M: Marker>(&mut self) {
        use bevy_ecs::entity::Entity;
        use bevy_ecs::system::Commands;
        if M::IS_ALL {
            eprintln!("despawn_with_marker should not be used to despawn all entities.");
            return;
        }
        self.run_system_once(|mut commands: Commands, query: Query<Entity, M::Query>| {
            for entity in query.iter() {
                commands.entity(entity).despawn()
            }
        })
    }
}

/// Resource that contains the path of file output.
#[derive(Debug, Clone, Resource)]
pub struct FileOutput<M: Marker>(String, PhantomData<M>);

#[cfg(feature="fs")]
impl<M: Marker> FileOutput<M> {
    pub fn new(s: impl Into<String>) -> Self{
        FileOutput(s.into(), PhantomData)
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}

/// Resource that contains the bytes output, unique for marker.
#[derive(Debug, Clone, Resource, Default)]
pub struct BytesOutput<M: Marker>(Vec<u8>, PhantomData<M>);

impl<M: Marker> BytesOutput<M> {
    pub fn new() -> Self{
        BytesOutput(Vec::new(), PhantomData)
    }

    pub fn get(&self) -> &[u8] {
        &self.0
    }

    pub fn take(self) -> Vec<u8> {
        self.0
    }
}

/// Resource that contains the string output, unique per marker.
/// 
/// Requires human readable format.
#[derive(Debug, Clone, Resource, Default)]
pub struct StringOutput<M: Marker>(String, PhantomData<M>);

impl<M: Marker> StringOutput<M> {
    pub fn new() -> Self{
        StringOutput(String::new(), PhantomData)
    }

    pub fn get(&self) -> &str {
        &self.0
    }

    pub fn take(self) -> String {
        self.0
    }
}

/// Resource that contains the path of file input, unique per marker.
#[derive(Debug, Clone, Resource)]
pub struct FileInput<M: Marker>(String, PhantomData<M>);

#[cfg(feature="fs")]
impl<M: Marker> FileInput<M> {
    pub fn new(s: impl Into<String>) -> Self{
        FileInput(s.into(), PhantomData)
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}

/// Resource that contains the bytes output, unique per marker.
#[derive(Debug, Clone, Resource, Default)]
pub struct BytesInput<M: Marker>(Vec<u8>, PhantomData<M>);

impl<M: Marker> BytesInput<M> {
    pub fn new(b: impl Into<Vec<u8>>) -> Self{
        BytesInput(b.into(), PhantomData)
    }

    pub fn get(&self) -> &[u8] {
        &self.0
    }

    pub fn take(self) -> Vec<u8> {
        self.0
    }
}
