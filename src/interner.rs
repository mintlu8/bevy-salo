
/// Create an integer based enum and a resource that manages its associated strings.
#[macro_export]
macro_rules! interned_enum {
    ($res: ident, $name: ident : $repr: ident {$($fields: ident),* $(,)*}) => {
        $crate::interned_enum!($res, $name : $repr {} (0) $($fields),*);
    };
    ($res: ident, $name: ident : $repr: ident {$($fields: ident= $value: expr),*} ($index: expr) $first: ident $(, $rest: ident)*) => {
        $crate::interned_enum!($res, $name : $repr {$($fields= $value,)* $first = $index} ($index + 1) $($rest),*);
    };
    ($res: ident, $name: ident : $repr: ident {$($fields: ident= $value: expr),*} ($index: expr)) => {
        $crate::interned_enum!($res, $name : $repr {$($fields=$value),*});
    };
    ($res: ident, $name: ident : $repr: ident {$($fields: ident= $value: expr),* $(,)*}) => {
        #[derive(Debug, Clone, ::bevy_ecs::system::Resource)]
        pub struct $res {
            flags: std::collections::HashMap<String, $repr>,
            names: Vec<String>,
        }

        impl ::std::default::Default for $res {
            fn default() -> Self{
                Self::new()
            }
        }

        impl $res {
            pub fn new() -> Self {
                Self {
                    flags: std::collections::HashMap::from([
                        $((stringify!($fields).to_owned(), $value)),*
                    ]),
                    names: vec![$(stringify!($fields).to_owned()),*]
                }
            }

            pub fn len(&self) -> $repr {
                self.names.len() as $repr
            }

            pub fn clear(&mut self) {
                *self = Self::new()
            }

            pub fn try_get(&self, s: &str) -> Option<$name> {
                self.flags.get(s).map(|v| $name(*v))
            }

            pub fn get(&mut self, s: &str) -> $name {
                let len = self.len();
                match self.flags.get(s) {
                    Some(v) => $name(*v),
                    None => {
                        self.flags.insert(s.to_owned(), len);
                        self.names.push(s.to_owned());
                        $name(len)
                    }
                }
            }

            pub fn as_str(&self, value: $name) -> &str {
                match self.names.get(value.value() as usize) {
                    Some(v) => &v,
                    None => panic!("Invalid enum variant {:?}.", value),
                }
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ::bevy_ecs::component::Component)]
        pub struct $name($repr);

        impl $name {
            $(
                #[allow(non_upper_case_globals)]
                pub const $fields: Self = Self($value);
            )*

            pub fn value(&self) -> $repr {
                self.0
            }
        }

    };
}

#[cfg(test)]
mod test {
    use bevy_ecs::{system::{Res, ResMut}, entity::Entity};

    use crate::EntityPath;

    interned_enum!(ElementsServer, Elements: u64 {
        Water, Earth, Fire, Air
    });

    impl crate::SaveLoad for Elements {
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
            _: &mut bevy_ecs::system::Commands,
            _: bevy_ecs::entity::Entity,
            _: impl FnMut(&mut bevy_ecs::system::Commands, &crate::EntityPath) -> bevy_ecs::entity::Entity, 
            res: &mut ResMut<'s, ElementsServer>
        ) -> Self {
            res.get(&de)
        }
    }
}

/// Create an integer based flags and a resource that manages its associated strings.
#[macro_export]
macro_rules! interned_flags {
    ($res: ident, $name: ident : $repr: ident {$($fields: ident),* $(,)*}) => {
        $crate::interned_flags!($res, $name : $repr {} (0) $($fields),*);
    };
    ($res: ident, $name: ident : $repr: ident {$($fields: ident= $value: expr),*} ($index: expr) $first: ident $(, $rest: ident)*) => {
        $crate::interned_flags!($res, $name : $repr {$($fields= $value,)* $first = $index} ($index + 1) $($rest),*);
    };
    ($res: ident, $name: ident : $repr: ident {$($fields: ident= $value: expr),*} ($index: expr)) => {
        $crate::interned_flags!($res, $name : $repr {$($fields=$value),*});
    };
    ($res: ident, $name: ident : $repr: ident {$($fields: ident= $value: expr),* $(,)*}) => {
        #[derive(Debug, Clone, ::bevy_ecs::system::Resource)]
        pub struct $res {
            flags: std::collections::HashMap<String, $repr>,
            names: Vec<String>,
        }

        impl $res {
            pub fn new() -> Self {
                Self {
                    flags: std::collections::HashMap::from([
                        $((stringify!($fields).to_owned(), $value)),*
                    ]),
                    names: vec![$(stringify!($fields).to_owned()),*]
                }
            }

            pub fn len(&self) -> $repr {
                self.names.len() as $repr
            }

            pub fn clear(&mut self) {
                *self = Self::new()
            }

            pub fn get_single(&mut self, s: &str) -> $name {
                let len = self.len();
                match self.flags.get(s) {
                    Some(v) => $name(1 << *v),
                    None => {
                        self.flags.insert(s.to_owned(), len);
                        self.names.push(s.to_owned());
                        $name(1 << len)
                    }
                }
            }

            pub fn try_get_single(&self, s: &str) -> Option<$name> {
                self.flags.get(s).map(|v| $name(1 << *v))
            }

            pub fn get(&mut self, s: &str) -> $name {
                s.split('|').map(|x| self.get_single(x)).fold($name::None, |a, b| a|b)
            }

            pub fn try_get(&self, s: &str) -> Option<$name> {
                s.split('|').map(|x| self.try_get_single(x)).fold(Some($name::None), |a, b| Some(a?|b?))
            }

            pub fn as_str(&self, value: $name) -> String {
                if value == $name::None {
                    return "None".to_owned()
                }
                let mut v = value.0;
                let mut index = 0;
                let mut result = Vec::new();
                while v > 0 {
                    if v & 1 == 1 {
                        let s = match self.names.get(index) {
                            Some(v) => v.as_str(),
                            None => panic!("Invalid enum variant {:?}.", value),
                        };
                        result.push(s);
                    }
                    v >>= 1;
                    index += 1;
                }
                result.join("|")
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name($repr);

        impl $name {
            #[allow(non_upper_case_globals)]
            pub const None: Self = Self(0);
            
            $(
                #[allow(non_upper_case_globals)]
                pub const $fields: Self = Self(1 << ($value));
            )*

            pub fn value(&self) -> $repr {
                self.0
            }

            pub fn contains(&self, other: Self) -> bool {
                self.0 & other.0 == other.0
            }

            pub fn intersects(&self, other: Self) -> bool {
                self.0 & other.0 > 0
            }
        }

        impl std::ops::Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self{
                Self(self.0 - (self.0 & rhs.0))
            }
        }

        impl std::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self{
                Self(self.0 | rhs.0)
            }
        }

        impl std::ops::BitAnd for $name {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self{
                Self(self.0 & rhs.0)
            }
        }

        impl std::ops::BitXor for $name {
            type Output = Self;
            fn bitxor(self, rhs: Self) -> Self{
                Self(self.0 ^ rhs.0)
            }
        }

        impl std::ops::SubAssign for $name {
            fn sub_assign(&mut self, rhs: Self){
                self.0 -= self.0 & rhs.0
            }
        }

        impl std::ops::BitOrAssign for $name {
            fn bitor_assign(&mut self, rhs: Self){
                self.0 |= rhs.0
            }
        }

        impl std::ops::BitAndAssign for $name {
            fn bitand_assign(&mut self, rhs: Self){
                self.0 &= rhs.0
            }
        }

        impl std::ops::BitXorAssign for $name {
            fn bitxor_assign(&mut self, rhs: Self){
                self.0 ^= rhs.0
            }
        }
    };
}