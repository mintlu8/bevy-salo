
use std::marker::PhantomData;
use bevy_ecs::world::World;
use bevy_ecs::schedule::{Schedule, IntoSystemConfigs};
use crate::methods::SerializationMethod;
use crate::{SaveLoad, StringOutput, BytesOutput, Marker};
use crate::schedules::*;

pub trait Sealed {}
pub trait MarkerSeal {}

use super::All;

impl All {
    pub fn new<S: SerializationMethod>() -> All<S>{
        All::<S>(PhantomData)
    }

    pub fn fork<S: SerializationMethod, const FORK: char>() -> All<S, FORK> {
        All::<S, FORK>(PhantomData)
    }
}

impl<S: SerializationMethod, const FORK: char> Default for All<S, FORK> {
    fn default() -> Self { All(PhantomData) }
}

pub trait Build {
    fn build<M: Marker>(ser: &mut Schedule, de: &mut Schedule, reset: &mut Schedule);
}

impl Build for () {
    fn build<M: Marker>(_: &mut Schedule, _: &mut Schedule, _: &mut Schedule) {}
}

macro_rules! build_tuple {
    ($first: ident) => {};
    ($first: ident, $($rest: ident),*) => {
        impl<$first: Build $(,$rest: Build)*> Build for ($first $(,$rest)*) {
            fn build<M: Marker>(ser: &mut Schedule, de: &mut Schedule, reset: &mut Schedule) {
                $first::build::<M>(ser, de, reset);
                $($rest::build::<M>(ser, de, reset);)*
            }
        }
        build_tuple!($($rest),*);
    };
}

build_tuple!(A,B,C,D,E,F,G);


impl<T> Build for T where T: SaveLoad {
    fn build<M: Marker>(ser: &mut Schedule, de: &mut Schedule, reset: &mut Schedule) {
        ser.add_systems(Self::build_path::<M>.in_set(InitSerialize));
        ser.add_systems(Self::serialize_system::<M>.in_set(RunSerialize));
        de.add_systems(Self::build_path::<M>.in_set(InitDeserialize));
        de.add_systems(Self::deserialize_system::<M>.in_set(RunDeserialize));
        reset.add_systems(Self::remove_all::<M>);
    }
}

pub trait SerializationResult: Sized {
    fn setup<M: Marker>(w: &mut World);
    fn get<M: Marker>(w: &mut World) -> Option<Self>;
    fn as_bytes(&self) -> &[u8];
}

impl SerializationResult for String {
    fn setup<M: Marker>(w: &mut World) {
        w.init_resource::<StringOutput<M>>();
    }
    fn get<M: Marker>(w: &mut World) -> Option<Self>{
        Some(w.remove_resource::<StringOutput<M>>()?.take())
    }
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl SerializationResult for Vec<u8> {
    fn setup<M: Marker>(w: &mut World) {
        w.init_resource::<BytesOutput<M>>();
    }
    fn get<M: Marker>(w: &mut World) -> Option<Self>{
        Some(w.remove_resource::<BytesOutput<M>>()?.take())
    }
    fn as_bytes(&self) -> &[u8] {
        &self
    }
}
