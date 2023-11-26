use std::{any::type_name, fmt::Debug};

use anyhow::Ok;
use serde::{de::DeserializeOwned, Serialize};

#[cfg(feature="fs")]
use std::{io::{BufWriter, BufReader}, fs::File};


pub trait SerializeValue: Serialize + DeserializeOwned + Clone + Default + Debug + Send + Sync + 'static {
    fn is_empty(&self) -> bool;
}

impl SerializeValue for Vec<u8> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl SerializeValue for serde_json::Value {
    fn is_empty(&self) -> bool {
        self.is_null()
    }
}

pub trait SerializationMethod: Debug + Send + Sync + 'static {
    type Value: SerializeValue;
    fn serialize_value(item: &impl serde::Serialize)-> anyhow::Result<Self::Value>;
    fn deserialize_value<T: DeserializeOwned>(item: Self::Value)-> anyhow::Result<T>;
    fn serialize_bytes(item: &impl serde::Serialize)-> anyhow::Result<Vec<u8>>;
    fn serialize_string(_item: &impl serde::Serialize)-> anyhow::Result<String> {
        anyhow::bail!("Format {} is not human-readable.", type_name::<Self>())
    }
    fn deserialize<T: DeserializeOwned>(item: &[u8]) -> anyhow::Result<T>;
    #[cfg(feature="fs")]
    fn serialize_file(file: &str, item: &impl serde::Serialize)-> anyhow::Result<()> {
        std::fs::write(file, Self::serialize_bytes(item)?)?;
        anyhow::Ok(())
    }
    #[cfg(feature="fs")]
    fn deserialize_file<T: DeserializeOwned>(file: &str)-> anyhow::Result<T> {
        let bytes = std::fs::read(file)?;
        Self::deserialize(&bytes)
    }
}

#[derive(Debug)]
pub struct SerdeJson<const PRETTY: bool=true>;

impl<const PRETTY: bool> SerializationMethod for SerdeJson<PRETTY> {
    type Value = serde_json::Value;
    fn serialize_value(item: &impl serde::Serialize)-> anyhow::Result<Self::Value>{
        Ok(serde_json::to_value(item)?)
    }
    fn deserialize_value<T: DeserializeOwned>(item: Self::Value)-> anyhow::Result<T>{
        Ok(serde_json::from_value(item)?)
    }
    fn serialize_bytes(item: &impl serde::Serialize) -> anyhow::Result<Vec<u8>> {
        Ok(if PRETTY {
            serde_json::to_string_pretty(item)?.into_bytes()
        } else {
            serde_json::to_string(item)?.into_bytes()
        })
    }
    fn serialize_string(item: &impl serde::Serialize)-> anyhow::Result<String> {
        Ok(if PRETTY {
            serde_json::to_string_pretty(item)?
        } else {
            serde_json::to_string(item)?
        })
    }
    fn deserialize<T: DeserializeOwned>(item: &[u8]) -> anyhow::Result<T>{
        Ok(serde_json::from_slice(item)?)
    }
    #[cfg(feature="fs")]
    fn serialize_file(file: &str, item: &impl serde::Serialize)-> anyhow::Result<()> {
        if PRETTY {
            serde_json::to_writer_pretty(BufWriter::new(File::create(file)?), item)?;
        } else {
            serde_json::to_writer(BufWriter::new(File::create(file)?), item)?;
        }
        Ok(())
    }
    #[cfg(feature="fs")]
    fn deserialize_file<'de, T: DeserializeOwned>(file: &str)-> anyhow::Result<T> {
        Ok(serde_json::from_reader(BufReader::new(File::open(file)?))?)
    }
}

#[cfg(feature="ron")]
#[derive(Debug)]
pub struct Ron<const PRETTY: bool=true>;

#[cfg(feature="ron")]
impl<const PRETTY: bool> SerializationMethod for Ron<PRETTY> {
    // ron::Value does not round trip and doesn't actually expand to the full ron syntax.
    // so we use serde_json for now.
    type Value = serde_json::Value;
    fn serialize_value(item: &impl serde::Serialize)-> anyhow::Result<Self::Value>{
        Ok(serde_json::to_value(item)?)
    }
    fn deserialize_value<T: DeserializeOwned>(item: Self::Value)-> anyhow::Result<T>{
        Ok(serde_json::from_value(item)?)
    }
    fn serialize_bytes(item: &impl serde::Serialize) -> anyhow::Result<Vec<u8>> {
        use ron::ser::PrettyConfig;
        Ok(if PRETTY {
            ron::ser::to_string_pretty(item, PrettyConfig::default())?.into_bytes()
        } else {
            ron::ser::to_string(item)?.into_bytes()
        })
    }
    fn serialize_string(item: &impl serde::Serialize)-> anyhow::Result<String> {
        use ron::ser::PrettyConfig;
        Ok(if PRETTY {
            ron::ser::to_string_pretty(item, PrettyConfig::default())?
        } else {
            ron::ser::to_string(item)?
        })
    }
    fn deserialize<T: DeserializeOwned>(item: &[u8]) -> anyhow::Result<T>{
        Ok(ron::from_str(std::str::from_utf8(item)?)?)
    }
    #[cfg(feature="fs")]
    fn serialize_file(file: &str, item: &impl serde::Serialize)-> anyhow::Result<()> {
        use ron::ser::PrettyConfig;
        if PRETTY {
            ron::ser::to_writer_pretty(BufWriter::new(File::create(file)?), item, PrettyConfig::default())?;
        } else {
            ron::ser::to_writer(BufWriter::new(File::create(file)?), item)?;
        }
        Ok(())
    }
    #[cfg(feature="fs")]
    fn deserialize_file<'de, T: DeserializeOwned>(file: &str)-> anyhow::Result<T> {
        Ok(ron::de::from_reader(BufReader::new(File::open(file)?))?)
    }
}

#[cfg(feature="postcard")]
#[derive(Debug)]
pub struct Postcard;

#[cfg(feature="postcard")]
impl SerializationMethod for Postcard {
    type Value = Vec<u8>;
    fn serialize_value(item: &impl serde::Serialize)-> anyhow::Result<Self::Value>{
        Ok(postcard::to_allocvec(item)?)
    }
    fn deserialize_value<T: DeserializeOwned>(item: Self::Value)-> anyhow::Result<T>{
        Ok(postcard::from_bytes(&item)?)
    }
    fn serialize_bytes(item: &impl serde::Serialize) -> anyhow::Result<Vec<u8>> {
        Ok(postcard::to_allocvec(item)?)
    }
    fn deserialize<T: DeserializeOwned>(item: &[u8]) -> anyhow::Result<T>{
        Ok(postcard::from_bytes(item)?)
    }
    #[cfg(feature="fs")]
    fn serialize_file(file: &str, item: &impl serde::Serialize)-> anyhow::Result<()> {
        postcard::to_io(item, BufWriter::new(File::create(file)?))?;
        Ok(())
    }
    #[cfg(feature="fs")]
    fn deserialize_file<'de, T: DeserializeOwned>(file: &str)-> anyhow::Result<T> {
        // basically a std::bufwriter
        Ok(postcard::from_io((File::open(file)?, &mut vec![0; 8 * 1024]))?.0)
    }
}
