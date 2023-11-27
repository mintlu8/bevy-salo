use std::borrow::Cow;

use bevy_ecs::{system::{Resource, SystemParam, Commands, Res, ResMut, StaticSystemParam, SystemParamItem}, entity::Entity};
use serde::{de::DeserializeOwned, Serialize};
use crate::{methods::SerializationMethod, PathedValue, EntityParent, DeserializeContext};
use crate::{Marker, SerializeContext, EntityPath};

/// Allows a resource to be saved and loaed with serde.
pub trait SaveLoadResCore: Serialize + DeserializeOwned + Resource + Sized {

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
}

impl<T> SaveLoadRes for T where T: SaveLoadResCore {
    type Ser<'ser> = &'ser Self;

    type De = Self;

    type Context<'w, 's> = ();

    type ContextMut<'w, 's> = ();

    fn to_serializable<'t>(&'t self, 
        _: impl Fn(Entity) -> EntityPath,
        _: &'t SystemParamItem<Self::Context<'_, '_>>
    ) -> Self::Ser<'t> {
        self
    }

    fn from_deserialize(
        de: Self::De, 
        _: &mut Commands,
        _: impl FnMut(&mut Commands, &EntityPath) -> Entity, 
        _: &mut SystemParamItem<Self::ContextMut<'_, '_>>
    ) -> Self {
        de
    }
    
    fn type_name() -> Cow<'static, str> {
        <Self as SaveLoadResCore>::type_name()
    }
}

/// The core trait for resources, allows a resource to be saved and loaed with context.
pub trait SaveLoadRes: Resource + Sized {
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
        path_fetcher: impl Fn(Entity) -> EntityPath,
        res: &'t SystemParamItem<Self::Context<'_, '_>>
    ) -> Self::Ser<'t>;

    /// Convert to a deserializable struct.
    /// 
    /// # Parameters
    /// 
    /// * entity_fetcher: This will either get or spawn an entity based on the query.
    fn from_deserialize(
        de: Self::De, 
        commands: &mut Commands,
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

    /// System for serialization.
    fn serialize_system<M: Marker>(
        mut paths: ResMut<SerializeContext<M>>,
        res: Option<Res<Self>>,
        ctx: StaticSystemParam<Self::Context<'_, '_>>,
    ) {
        if let Some(res) = res {
            let path_fetcher = |e: Entity| {
                match paths.paths.get(&e) {
                    Some(path) => EntityPath::Path(path.clone()),
                    None => EntityPath::Entity(e.to_bits()),
                }
            };
            let value = match M::Method::serialize_value(&res.to_serializable(path_fetcher, &ctx)) {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("{}", e);
                    return;
                }
            };
            if paths.components.insert(Self::type_name().clone(), vec![PathedValue {
                parent: EntityParent::Root,
                path: EntityPath::Unique,
                value
            }]).is_some() {
                panic!("Duplicate resource: {}.", Self::type_name())
            }

        }
    }

    /// System for deserialization.
    fn deserialize_system<M: Marker>(
        mut commands: Commands,
        mut context: ResMut<DeserializeContext<M>>,
        mut ctx_mut: StaticSystemParam<Self::ContextMut<'_, '_>>,
    ) {
        let Some(mut items) = context.components.remove(Self::type_name().as_ref()) else {return};
        let Some(PathedValue { parent:_, path:_, value }) = items.pop() else {return};
        let None = items.pop() else { panic!("Found multiple items for a resource, expected 0 or 1.")};
        let de = match M::Method::deserialize_value(value) { 
            Ok(de) => de,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };

        let ctx_fetch = |commands: &mut Commands, path: &EntityPath| {
            match context.path_map.get(path) {
                Some(entity) => *entity,
                None => commands.spawn_empty().id()
            }
        };
        let res = Self::from_deserialize(de, &mut commands, ctx_fetch, &mut ctx_mut);
        commands.insert_resource(res)
    }

    /// Remove this resource.
    fn remove<M: Marker>(mut commands: Commands) {
        commands.remove_resource::<Self>()
    }

}