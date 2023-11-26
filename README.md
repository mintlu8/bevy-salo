# bevy-salo

bevy_salo (SAveLOad) is an ECS based serialization crate for bevy_ecs.

## Unique Features

* Not dependent on reflection or bevy_app.
* Greater user control.
* Match entities by name.
* Custom ser/de methods that can load resources, spawn entities, etc.

## Getting Started

To get started, register the plugin and all the types you need to serialize.

```rust
// It is recommanded to alias here.
type All = bevy_salo::All<SerdeJson>;
app.add_plugins(
    SaveLoadPlugin::new::<All>()
        .register::<Unit>()
        .register::<Weapon>()
        .register::<Stat>()
        .register::<Hp>()
);
```

Generic types (unforunately) need to be registered separately.

```rust
SaveLoadPlugin::new::<All>()
    .register::<Unit<Human>>()
    .register::<Unit<Monster>>()
```

`All` serializes all entities, to narrow the scope with a marker component:

```rust
#[derive(Debug, Default, Component)]
pub struct SaLo;

impl bevy_salo::MarkerComponent for SaLo {
    // Set the serialization method here.
    type Method = SerdeJson;
}

app.add_plugins(
    SaveLoadPlugin::new::<SaLo>()
        .register::<Unit>()
);
```

## Usage

`bevy_salo` creates schedules for
serialization and deserialization. If you have access to a `&mut World`,
you can use these extension methods. You can either use a system or
implement a custom `Command`.

```rust
world.load_from_file::<All>("test.ron");
world.save_to_file::<All>("test.json");
world.deserialize_from::<All>(bytes);
let bytes = world.serialize_to::<All>();
```

Deserialize does not remove existing items.
To cleanup, choose one of these functions
that best suit your use case, or write your own logic.

```rust
// remove all serialized components, does not despawn entities
world.remove_serialized_components::<All>();

// despawn entities with a marker.
world.despawn_with_marker::<Marker>();
```

## Traits

For your structs to work with `bevy_salo`, you need to implement one of three traits:
`SaveLoadCore`, `SaveLoadMapped` and `SaveLoad`.

### SaveLoadCore

`SaveLoadCore` can be easily implemented on any struct implementing
`serde::Serialize` and `serde::Deserialize`.

```rust
struct Weapon {
    name: String,
    damage: f32,
    cost: i32,
}
impl SaveLoadCore for Weapon {}
```

However you should almost always overwrite the `type_name` function on `SaveLoadCore`,
since the default implementation `Any::type_name()` is unstable across rust verions and
namespace dependent, which could break the save format when refactoring.

```rust
impl SaveLoadCore for Weapon {
    // This has to be unique across all registered types.
    fn type_name() -> Cow<'static, str> {
        Cow::Borrowed("weapon")
    }
    // Provide a path name for the associated entity.
    fn path_name(&self) -> Option<Cow<'static, str>> {
        Some(self.name.clone().into())
    }
}
```

## SaveLoadMapped

`SaveLoadMapped` is just like `SaveLoadCore` but you can map non-serializable struct into
a serializable.

## SaveLoad

Implementing `SaveLoad` allows you to do arbitrary things during
serialization and deserialization. Checkout its documentation for more information.

String interning example:

```rust
interned_enum!(ElementsServer, Elements: u64 {
    Water, Earth, Fire, Air
});

impl SaveLoad for Elements {
    type Ser<'ser> = &'ser str;
    type De = String;
    type Context<'w, 's> = Res<'w, ElementsServer>;
    type ContextMut<'w, 's> = ResMut<'s, ElementsServer>;

    fn to_serializable<'t, 'w, 's>(&'t self, 
        _: Entity,
        _: impl Fn(Entity) -> EntityPath, 
        res: &'t Res<'w, ElementsServer>
    ) -> Self::Ser<'t> {
        res.as_str(*self)
    }

    fn from_deserialize<'w, 's>(
        de: Self::De, 
        _: &mut Commands,
        _: Entity,
        _: impl FnMut(&mut Commands, &EntityPath) -> Entity, 
        res: &mut ResMut<'s, ElementsServer>
    ) -> Self {
        res.get(&de)
    }
}
```

## Paths

`bevy_salo` records each entity as either its Entity ID or its path.
Entity ID is only used for disambiguation,
while path allow matching with existing entity.

Each component can optionally provide a name with the `path_name` function
defined in the aforementioned traits for their associated entity.
The `PathName` component can be used instead for non-serialized entities.

In this example
the entity has the path name `"John"`.

```rust
Entity {
    Character => Some("John"),
    Weapon => None,
    Armor => None,
}
```

This panics for conflicting names.

```rust
Entity {
    Character => Some("John"),
    Role => Some("Protagonist"),
}
```

An entity's path contains all its named ancestors. Consider this entity:

```rust
(root)::window::(unnamed)::characters::John::weapon
```

The weapon's path is `characters::John::weapon`, while everything before its
unnamed ancestor is ignored. This is helpful when you want to insert `"John"`
into an existing entity `"characters"`.

Pathed entities must have unique paths, but duplicated names are allowed.

```rust
// legal, although both named `weapon`, paths are different
characters::John::weapon
characters::Jane::weapon

// illegal, 2 entities with path `characters`
characters::John::weapon
characters::Jane::(unnamed)::characters
```

When serializing, non-serializing parents of
serialized children must be named.

```rust
// legal, parent is root
(root)::[Named]

// legal, parent is named
Named::[Named]

// illegal, parent is not named, cannot deserialize correctly
(unnamed)::[Named]
```
