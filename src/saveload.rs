
use std::borrow::Cow;
use std::collections::HashMap;
use std::marker::PhantomData;

use bevy_ecs::{component::Component, entity::Entity, query::With};
use bevy_ecs::system::{Query, Resource, ResMut, Commands, SystemParam, SystemParamItem, StaticSystemParam};
use bevy_hierarchy::{Parent, BuildChildren};
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use crate::methods::SerializationMethod;
use crate::Marker;

/// This collects names from various sources to build paths.
#[derive(Debug, Resource, Default)]
pub struct PathNames<M: Marker>(HashMap<Entity, Cow<'static, str>>, PhantomData<M>);

impl<M: Marker> PathNames<M> {
    pub fn push(&mut self, entity: Entity, name: Cow<'static, str>) {
        match self.0.get_mut(&entity) {
            Some(n) => if n != &name {
                panic!("Trying to rename entity {:?} from {} to {}.", entity, n, name);
            },
            None => {
                self.0.insert(entity, name);
            },
        }
    }

    pub fn get(&self, e: Entity) -> Option<&str>{
        self.0.get(&e).map(|x| x.as_ref())
    }

    pub fn iter(&self) -> impl IntoIterator<Item = (Entity, &str)>{
        self.0.iter().map(|(k, v)| (*k, v.as_ref()))
    }
}

type PathedValueOf<M> = PathedValue<<<M as Marker>::Method as SerializationMethod>::Value>;

/// Paths used in the serialization step.
#[derive(Debug, Resource, Default)]
pub struct SerializeContext<M: Marker>{
    pub(crate) paths: HashMap<Entity, String>,
    pub(crate) components: HashMap<Cow<'static, str>, Vec<PathedValueOf<M>>>,
    p: PhantomData<M>
}

impl<M: Marker> SerializeContext<M> {
    pub fn serialized(&self) -> &impl serde::Serialize {
        &self.components
    }

}

/// Paths used in the deserialization step.
#[derive(Debug, Resource, Default)]
pub struct DeserializeContext<M: Marker>{
    pub(crate) components: HashMap<String, Vec<PathedValueOf<M>>>,
    pub(crate) path_map: HashMap<EntityPath, Entity>,
    p: PhantomData<M>,
}

impl<M: Marker> DeserializeContext<M> {
    pub(crate) fn load(&mut self, components: HashMap<String, Vec<PathedValueOf<M>>>) {
        self.components = components;
    }

    pub fn get_or_new(&mut self, commands: &mut Commands, path: &EntityPath) -> Entity {
        match path {
            EntityPath::Unique => commands.spawn_empty().id(),
            _ => match self.path_map.get(path) {
                Some(entity) => *entity,
                None => {
                    let id = commands.spawn_empty().id();
                    self.path_map.insert(path.clone(), id);
                    id
                }
            }
        }
    }

    pub fn push(&mut self, entity: Entity, path: &str) {
        if let Some(prev) = self.path_map.insert(EntityPath::Path(path.into()), entity) {
            if prev != entity {
                panic!("Duplicate path {} for entity {:?} and {:?}", path, prev, entity)
            }
        };
    }   

}

#[derive(Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub(crate) enum EntityParent {
    #[default]
    Root,
    Path(String),
    Entity(u64),
}

/// Path of an entity. Either an entity number or a joined path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum EntityPath {
    /// Unused when serializing. 
    /// 
    /// In handwritten inputs, 
    /// empty paths always provides a new entity.
    #[default]
    Unique,
    Entity(u64),
    Path(String),
}

impl EntityPath {
    pub fn is_unique(&self) -> bool {
        self == &Self::Unique
    }

    /// Get the last `::` delimited segment of path
    /// 
    /// # Panics
    /// 
    /// If `self` is not a path.
    pub fn name(&self) -> &str {
        match self {
            EntityPath::Unique => panic!("Empty path does not contain a name."),
            EntityPath::Entity(e) => panic!("Entity {:?} does not contain a name.", e),
            EntityPath::Path(p) => match p.rsplit_once("::") {
                Some((_, a)) => a,
                None => p,
            },
        }
    }

    /// Get the last `::` delimited segment of path
    pub fn get_name(&self) -> Option<&str> {
        match self {
            EntityPath::Path(p) => match p.rsplit_once("::") {
                Some((_, a)) => Some(a),
                None => Some(p),
            },
            _ => None,
        }
    }
}

impl From<EntityParent> for EntityPath {
    fn from(value: EntityParent) -> Self {
        match value {
            EntityParent::Root => panic!("Root is not a valid owned path."),
            EntityParent::Path(p) => EntityPath::Path(p),
            EntityParent::Entity(e) => EntityPath::Entity(e),
        }
    }
}


#[derive(Debug)]
pub(crate) struct PathedValue<V>{
    pub(crate) parent: EntityParent,
    pub(crate) path: EntityPath,
    pub(crate) value: V,
}

/// The core trait, allows a component to be saved and loaed with context.
pub trait SaveLoad: Component + Sized {
    type Ser<'ser>: serde::Serialize;
    type De: serde::de::DeserializeOwned;

    type Context<'w, 's>: SystemParam; 
    type ContextMut<'w, 's>: SystemParam;

    /// Convert to a serializable struct.
    /// 
    /// # Parameters
    /// 
    /// * path_fetcher: Convert entity to path if exists.
    fn to_serializable<'t>(&'t self, 
        entity: Entity,
        path_fetcher: impl Fn(Entity) -> EntityPath,
        res: &'t SystemParamItem<Self::Context<'_, '_>>
    ) -> Self::Ser<'t>;

    /// Inplement this if: 
    /// 
    /// * You need to add additional components or spawn children derived from this component.
    /// * You need to fetch resources from the `World`.
    /// 
    /// # Rules
    /// 
    /// Same rules with schedules go here, you cannot access anything applied in the deserialization step with this function.
    /// 
    /// # Parameters
    /// 
    /// * entity_fetcher: This will either get or spawn an entity based on the query.
    fn from_deserialize(
        de: Self::De, 
        commands: &mut Commands,
        self_entity: Entity,
        entity_fetcher: impl FnMut(&mut Commands, &EntityPath) -> Entity, 
        ctx: &mut SystemParamItem<Self::ContextMut<'_, '_>>
    ) -> Self;

    /// Name associated with this type. 
    /// This is used in deserialization
    /// and must be unique accross for all generics.
    /// 
    /// The default implementation is `Any::type_name`, 
    /// which is unstable according to documentation, a bit verbose,
    /// and might break if you move namespaces around. It is recommended to implement this.
    fn type_name() -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<Self>())
    }

    /// Provide a locally unique name for the assiciated entity. 
    /// This builds a path with all its
    /// named ancestors, which provides interopability.
    /// 
    /// `::` is reserved for path separation, be careful when using it here.
    /// 
    /// # Panics
    /// 
    /// When trying to assign a conflicting name.
    fn path_name(&self) -> Option<Cow<'static, str>> {
        None
    }

    /// Set the path name for the current entity if `path_name` is not none.
    fn build_path<M: Marker>(
        mut paths: ResMut<PathNames<M>>,
        query: Query<(Entity, &Self), M::Query>, 
    ) {
        for (entity, item) in query.iter() {
            if let Some(path) = item.path_name() {
                paths.push(entity, path);
            }
        }
    }

    /// System for serialization.
    fn serialize_system<M: Marker>(
        mut paths: ResMut<SerializeContext<M>>,
        query: Query<(Entity, &Self), M::Query>, 
        parents: Query<&Parent>,
        marked: Query<(), M::Query>,
        ctx: StaticSystemParam<Self::Context<'_, '_>>,
    ) {
        for (entity, item) in query.iter() {
            let parent = match parents.get(entity) {
                Ok(parent) => {
                    if let Some(path) = paths.paths.get(&parent.get()) {
                        EntityParent::Path(path.clone())
                    } else if marked.contains(parent.get()) {
                        EntityParent::Entity(parent.to_bits())
                    } else {
                        panic!("Trying to serialize component {} in orphaned entity {:?}. \
                            Parent {:?} is neither serialized nor named.",
                            Self::type_name(),
                            entity,
                            parent.get()
                        );
                    }
                },
                Err(_) => EntityParent::Root,
            };
            let path = if let Some(name) = paths.paths.get(&entity) {
                EntityPath::Path(name.clone())
            } else {
                EntityPath::Entity(entity.to_bits())
            };
            let path_fetcher = |e: Entity| {
                match paths.paths.get(&e) {
                    Some(path) => EntityPath::Path(path.clone()),
                    None => EntityPath::Entity(e.to_bits()),
                }
            };
            let path = PathedValue {
                parent, 
                path,
                value: M::Method::serialize_value(&Self::to_serializable(item, entity, path_fetcher, &ctx)).unwrap()
            };
            match paths.components.get_mut(&Self::type_name()) {
                Some(vec) => vec.push(path),
                None => { 
                    paths.components.insert(
                        Self::type_name().clone(), 
                        vec![path],
                    );
                }
            }
        }
    }

    /// System for deserialization.
    fn deserialize_system<M: Marker>(
        mut commands: Commands,
        mut context: ResMut<DeserializeContext<M>>,
        mut ctx_mut: StaticSystemParam<Self::ContextMut<'_, '_>>,
    ) {
        let Some(items) = context.components.remove(Self::type_name().as_ref()) else {return};
        for PathedValue { parent, path, value } in items {
            
            let entity = match context.path_map.get(&path) {
                Some(entity) => {
                    commands.entity(*entity).id()
                },
                None => {
                    let e = commands.spawn_empty().id();
                    context.path_map.insert(path, e);
                    e
                }
            };
            let ctx_fetch = |commands: &mut Commands, path: &EntityPath| {
                match context.path_map.get(path) {
                    Some(entity) => *entity,
                    None => commands.spawn_empty().id()
                }
            };

            let item = Self::from_deserialize(
                M::Method::deserialize_value(value).unwrap(), 
                &mut commands,
                entity,
                ctx_fetch, 
                &mut ctx_mut
            );
            commands.entity(entity).insert(item);
            match parent {
                EntityParent::Root => (),
                p => {
                    let p = p.into();
                    let parent = match context.path_map.get(&p) {
                        Some(entity) => *entity,
                        None => commands.spawn_empty().id()
                    };
                    commands.entity(parent).add_child(entity);
                }
            }
        }
    }

    /// Remove all copies of the component.
    ///
    /// # Note 
    /// 
    /// This is invoked by `ResetSchedule`, will not be auto-runned by `LoadSchedule`.
    fn remove_all<M: Marker>(mut commands: Commands, entities: Query<Entity, (With<Self>, M::Query)>) {
        entities.iter().for_each(|e| {
            commands.entity(e).remove::<Self>();
        })
    }

}

/// Uses serde implementation directly with no additional requirements.
pub trait SaveLoadCore: Serialize + DeserializeOwned + Component {
    /// Type name of the struct, must be unique.
    fn type_name() -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<Self>())
    }

    /// Provide a locally unique name, this builds a path with its
    /// named ancestors, which provides interopability.
    /// 
    /// `::` is reserved for path separation, be careful when using it here.
    fn path_name(&self) -> Option<Cow<'static, str>> {
        None
    }
}

impl<T> SaveLoadMapped for T where T: SaveLoadCore {
    type Ser<'ser> = &'ser Self;
    type De = Self;

    fn type_name() -> Cow<'static, str> {
        <Self as SaveLoadCore>::type_name()
    }
    fn path_name(&self) -> Option<Cow<'static, str>> {
        <Self as SaveLoadCore>::path_name(self)
    }

    fn to_serializable(&self) -> Self::Ser<'_> { self }

    fn from_deserialize(de: Self::De) -> Self { de }

}

/// Use the serde implementation of a mapped struct(s).
pub trait SaveLoadMapped: Serialize + DeserializeOwned + Component {
    type Ser<'ser>: Serialize;
    type De: DeserializeOwned;
    fn to_serializable(&self) -> Self::Ser<'_>;

    fn from_deserialize(de: Self::De) -> Self;

    /// Name associated with this type. 
    /// This is used in deserialization
    /// and must be unique accross for all generics.
    /// 
    /// The default implementation is `Any::type_name`, 
    /// which is unstable according to its documentation, a bit verbose,
    /// and might break if you move namespaces around. It is recommended to implement this.
    fn type_name() -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<Self>())
    }

    /// Provide a locally unique name, this builds a path with its
    /// named ancestors, which provides interopability.
    /// 
    /// `::` is reserved for path separation, be careful when using it here.
    fn path_name(&self) -> Option<Cow<'static, str>> {
        None
    }
}

impl<T> SaveLoad for T where T: SaveLoadMapped {
    type Ser<'ser> = <Self as SaveLoadMapped>::Ser<'ser>;
    type De = <Self as SaveLoadMapped>::De;
    type Context<'w, 's> = ();
    type ContextMut<'w, 's> = ();

    fn type_name() -> Cow<'static, str> {
        <Self as SaveLoadMapped>::type_name()
    }

    fn path_name(&self) -> Option<Cow<'static, str>> {
        <Self as SaveLoadMapped>::path_name(self)
    }

    fn to_serializable<'t>(&'t self, 
        _: Entity,
        _: impl Fn(Entity) -> EntityPath, 
        _: &'t SystemParamItem<Self::Context<'_, '_>>) -> Self::Ser<'t>{
        <Self as SaveLoadMapped>::to_serializable(self)
    }

    fn from_deserialize(de: Self::De, 
        _: &mut Commands,
        _: Entity,
        _: impl FnMut(&mut Commands, &EntityPath) -> Entity, 
        _: &mut SystemParamItem<Self::ContextMut<'_, '_>>) -> Self{
        <Self as SaveLoadMapped>::from_deserialize(de)
    }
}

