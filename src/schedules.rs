use std::marker::PhantomData;

use bevy_ecs::entity::Entity;
use bevy_ecs::schedule::{ScheduleLabel, SystemSet, Schedule, IntoSystemConfigs};
use bevy_ecs::system::{Res, ResMut, Query};
use bevy_ecs::world::World;
use bevy_ecs::schedule::IntoSystemSetConfigs;
use bevy_hierarchy::Parent;
use crate::methods::SerializationMethod;
use crate::{SaveLoadPlugin, SaveLoad, PathNames, SerializeContext, DeserializeContext, BytesOutput, StringOutput, PathName, BytesInput};
use crate::sealed::Build;
use crate::{Marker, All};
use std::fmt::Debug;
use std::hash::Hash;

use crate::FileInput;

macro_rules! schedules {
    ($($names: ident),* $(,)?) => {
        $(
            #[derive(ScheduleLabel)]
            pub struct $names<M: Marker=All>(PhantomData<M>);

            impl $names {
                pub fn new() -> Self {
                    Self(PhantomData)
                }

                pub fn with_marker<M: Marker>() -> $names<M> {
                    $names(PhantomData)
                }
            }

            impl<M: Marker> Default for $names<M> {
                fn default() -> Self {
                    Self(PhantomData)
                }
            }

            impl<M: Marker> Debug for $names<M> {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}<{}>", stringify!($names), std::any::type_name::<M>())
                }
            }

            impl<M: Marker> Clone for $names<M> {
                fn clone(&self) -> Self {
                    Self(PhantomData)
                }
            }
            impl<M: Marker> Copy for $names<M> {}

            impl<M: Marker> PartialEq for $names<M> {
                fn eq(&self, _: &Self) -> bool { true }
            }

            impl<M: Marker> Eq for $names<M> {}

            impl<M: Marker> Hash for $names<M> {
                fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                    self.0.hash(state);
                }
            }
        )*
    };
}

macro_rules! system_sets {
    ($($names: ident),* $(,)?) => {
        $(#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
        pub struct $names;)*
    }
}


fn init_serialize<M: Marker>(w: &mut World) {
    w.remove_resource::<PathNames<M>>();
    w.init_resource::<PathNames<M>>();
    w.remove_resource::<SerializeContext<M>>();
    w.init_resource::<SerializeContext<M>>();
}

fn init_deserialize<M: Marker>(w: &mut World) {
    w.remove_resource::<PathNames<M>>();
    w.init_resource::<PathNames<M>>();
    w.remove_resource::<DeserializeContext<M>>();
    w.init_resource::<DeserializeContext<M>>();
}

#[cfg(feature="fs")]
fn write_to_file<M: Marker>(file: Option<Res<crate::FileOutput<M>>>, data: Res<SerializeContext<M>>) {
    if let Some(fo) = file {
        match M::Method::serialize_file(&fo.0, data.serialized()) {
            Ok(_) => (),
            Err(e) => eprintln!("Serialization failed: {}", e),
        }
    }
}

fn write_to_bytes<M: Marker>(
    buffer: Option<ResMut<BytesOutput<M>>>,
    data: Res<SerializeContext<M>>
) {
    if let Some(mut buffer) = buffer {
        match M::Method::serialize_bytes(data.serialized()) {
            Ok(bytes) => buffer.0 = bytes,
            Err(e) => eprintln!("Serialization failed: {}", e),
        }
    }
}

fn write_to_string<M: Marker>(
    buffer: Option<ResMut<StringOutput<M>>>, 
    data: Res<SerializeContext<M>>
) {
    if let Some(mut buffer) = buffer {
        match M::Method::serialize_string(data.serialized()) {
            Ok(bytes) => buffer.0 = bytes,
            Err(e) => eprintln!("Serialization failed: {}", e),
        }
    }
}

fn build_names<M: Marker>(mut res: ResMut<PathNames<M>>, names: Query<(Entity, &PathName)>) {
    for (entity, name) in names.iter() {
        res.push(entity, name.get())
    }
}

fn build_ser_context<M: Marker>(
    names: ResMut<PathNames<M>>, 
    mut ctx: ResMut<SerializeContext<M>>, 
    parents: Query<&Parent>
) {
    for (original, name) in names.iter() {
        let mut entity = original;
        let mut path = vec![name];
        while let Ok(parent) = parents.get(entity) {
            entity = parent.get();
            if let Some(name) = names.get(entity) {
                path.push(name);
            } else {
                break;
            }
        }
        path.reverse();
        ctx.paths.insert(original, path.join("::"));
    }
}

fn build_de_context<M: Marker>(
    names: ResMut<PathNames<M>>,
    file: Option<ResMut<FileInput<M>>>, 
    bytes: Option<Res<BytesInput<M>>>, 
    mut ctx: ResMut<DeserializeContext<M>>,
    parents: Query<&Parent>
) {
    match (file, bytes) {
        (Some(_), Some(_)) => {
            eprintln!("FileInput and BytesInput both exists, pick only one.");
        },
        #[cfg(feature="fs")]
        (Some(file), None) => {
            ctx.load(match M::Method::deserialize_file(file.get()) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Deserialization Failed: {}", e);
                    return;
                },
            });
        },
        (None, Some(bytes)) => {
            ctx.load(match M::Method::deserialize(bytes.get()) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Deserialization Failed: {}", e);
                    return;
                },
            });
        }
        _ => {
            eprintln!("No input found in deserialization.")
        },
    }

    for (original, name) in names.iter() {
        let mut entity = original;
        let mut path = vec![name];
        while let Ok(parent) = parents.get(entity) {
            entity = parent.get();
            if let Some(name) = names.get(entity) {
                path.push(name);
            } else {
                break;
            }
        }
        path.reverse();
        ctx.push(original, &path.join("::"));
    }
}


schedules!(SaveSchedule, LoadSchedule, ResetSchedule);
system_sets!(InitSerialize, RunSerialize, InitDeserialize, RunDeserialize, WriteOutput);

impl<M: Marker, C: Build> SaveLoadPlugin<M, C> {
    pub fn build_world(&self, world: &mut World) {
        let mut ser = Schedule::new(SaveSchedule::<M>(PhantomData));
        let mut de = Schedule::new(LoadSchedule::<M>(PhantomData));
        let mut reset = Schedule::new(ResetSchedule::<M>(PhantomData));
        ser.add_systems(init_serialize::<M>);
        ser.configure_sets(InitSerialize.after(init_serialize::<M>));
        ser.add_systems(build_ser_context::<M>.after(InitSerialize));
        ser.configure_sets(RunSerialize.after(build_ser_context::<M>));
        ser.configure_sets(WriteOutput.after(RunSerialize));
        ser.add_systems(build_names::<M>.in_set(InitSerialize));
        ser.add_systems((
            #[cfg(feature="fs")] write_to_file::<M>, 
            write_to_bytes::<M>, write_to_string::<M>
        ).in_set(WriteOutput));
        de.add_systems(init_deserialize::<M>);
        de.configure_sets(InitDeserialize.after(init_deserialize::<M>));
        de.add_systems(build_de_context::<M>.after(InitDeserialize));
        de.configure_sets(RunDeserialize.after(build_de_context::<M>));
        de.add_systems(build_names::<M>.in_set(InitDeserialize));
        C::build::<M>(&mut ser, &mut de, &mut reset);
        world.add_schedule(ser);
        world.add_schedule(de);
        world.add_schedule(reset);
    }

    pub fn register<T: SaveLoad>(self) -> SaveLoadPlugin<M, (C, T)> {
        SaveLoadPlugin(PhantomData)
    }
}

#[cfg(feature="bevy_app")]
impl<M: Marker, C: Build> bevy_app::Plugin for SaveLoadPlugin<M, C> where Self: Send + Sync + 'static  {
    fn build(&self, app: &mut bevy_app::App) {
        self.build_world(&mut app.world)
    }
}