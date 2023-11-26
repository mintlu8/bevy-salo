use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{PathedValue, EntityParent, EntityPath, methods::SerializeValue};

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum EntityPathUntagged<'t> {
    #[default]
    None,
    Entity(u64),
    Path(Cow<'t, str>)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EntityPathTagged<'t> {
    Unique,
    Entity(u64),
    Path(Cow<'t, str>)
}

impl EntityPathUntagged<'_> {
    pub fn is_default(&self) -> bool {
        self == &Self::None
    }
}

fn cow_is_default<'t>(v: &Cow<'t, impl SerializeValue>) -> bool{
    v.as_ref().is_empty()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(bound="")]
struct PathedValueSer<'t, V: SerializeValue>{
    #[serde(default, skip_serializing_if="EntityPathUntagged::is_default")]
    parent: EntityPathUntagged<'t>,
    #[serde(default, skip_serializing_if="EntityPathUntagged::is_default")]
    path: EntityPathUntagged<'t>,
    #[serde(default, skip_serializing_if="cow_is_default")]
    value: Cow<'t, V>,
}

impl<'t> From<&'t EntityParent> for EntityPathUntagged<'t> {
    fn from(value: &'t EntityParent) -> Self {
        match value {
            EntityParent::Root => Self::None,
            EntityParent::Path(p) => Self::Path(Cow::Borrowed(&p)),
            EntityParent::Entity(e) => Self::Entity(*e),
        }
    }
}

impl<'t> From<&'t EntityPath> for EntityPathUntagged<'t> {
    fn from(value: &'t EntityPath) -> Self {
        match value {
            EntityPath::Unique => Self::None,
            EntityPath::Path(p) => Self::Path(Cow::Borrowed(&p)),
            EntityPath::Entity(e) => Self::Entity(*e),
        }
    }
}

impl<'t> From<&'t EntityPath> for EntityPathTagged<'t> {
    fn from(value: &'t EntityPath) -> Self {
        match value {
            EntityPath::Unique => Self::Unique,
            EntityPath::Path(p) => Self::Path(Cow::Borrowed(&p)),
            EntityPath::Entity(e) => Self::Entity(*e),
        }
    }
}

impl<'t> From<EntityPathUntagged<'t>> for EntityParent {
    fn from(value: EntityPathUntagged<'t>) -> Self {
        match value {
            EntityPathUntagged::None => Self::Root,
            EntityPathUntagged::Path(p) => Self::Path(p.into_owned()),
            EntityPathUntagged::Entity(e) => Self::Entity(e),
        }
    }
}

impl<'t> From<EntityPathUntagged<'t>> for EntityPath {
    fn from(value: EntityPathUntagged<'t>) -> Self {
        match value {
            EntityPathUntagged::None => Self::Unique,
            EntityPathUntagged::Path(p) => Self::Path(p.into_owned()),
            EntityPathUntagged::Entity(e) => Self::Entity(e),
        }
    }
}

impl<'t> From<EntityPathTagged<'t>> for EntityPath {
    fn from(value: EntityPathTagged<'t>) -> Self {
        match value {
            EntityPathTagged::Unique => Self::Unique,
            EntityPathTagged::Path(p) => Self::Path(p.into_owned()),
            EntityPathTagged::Entity(e) => Self::Entity(e),
        }
    }
}

impl serde::Serialize for EntityPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        if serializer.is_human_readable() {
            EntityPathUntagged::from(self).serialize(serializer)
        } else {
            EntityPathTagged::from(self).serialize(serializer)
        }
    }
}

impl<'de> serde::Deserialize<'de> for EntityPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        if deserializer.is_human_readable() {
            Ok(EntityPathUntagged::deserialize(deserializer)?.into())
        } else {
            Ok(EntityPathTagged::deserialize(deserializer)?.into())
        }
    }
}

impl<V: SerializeValue> serde::Serialize for PathedValue<V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        use serde::ser::SerializeTuple;
        if serializer.is_human_readable() {
            PathedValueSer {
                parent: (&self.parent).into(),
                path: (&self.path).into(),
                value: Cow::Borrowed(&self.value),
            }.serialize(serializer)
        } else {
            let mut map = serializer.serialize_tuple(3)?;
            map.serialize_element(&self.parent)?;
            map.serialize_element(&self.path)?;
            map.serialize_element(&self.value)?;
            map.end()
        }   
    }
}



impl<'de, V: SerializeValue> serde::Deserialize<'de> for PathedValue<V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        if deserializer.is_human_readable() {
            let v: PathedValueSer<'_, V> = PathedValueSer::deserialize(deserializer)?;
            Ok(Self { 
                parent: v.parent.into(), 
                path: v.path.into(), 
                value: v.value.into_owned(), 
            })
        } else {
            let (parent, path, value) = <(EntityParent, EntityPath, V)>::deserialize(deserializer)?;
            Ok(Self { parent, path, value })
        }
    }
}