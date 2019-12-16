use crate::game::ConnectionState;

macro_rules! impl_packets {
    ($enum_name:ident, $type_enum_name:ident, $($state:ident, $bound_to:ident, $id:expr, $packet_name:ident { $($variant_body_field_name:ident: $variant_body_field_type:ty,)* },)*) => {
        #[derive(Debug, Clone, Copy)]
        pub enum BoundTo {
            Server,
            Client,
        }

        fn __type_check() {
            $(let _ = BoundTo::$bound_to;)*
        }

        #[derive(Debug, Clone)]
        pub enum $enum_name {
            $($packet_name { $($variant_body_field_name: $variant_body_field_type,)* },)*
        }

        impl $enum_name {
            pub fn ty(&self) -> $type_enum_name {
                match self {
                     $($enum_name::$packet_name {..} => $type_enum_name::$packet_name,)*
                }
            }
        }

        #[derive(Copy, Clone, Debug)]
        pub enum $type_enum_name {
            $($packet_name,)*
        }

        impl $type_enum_name {
            pub fn id(&self) -> i32 {
                match self {
                     $($type_enum_name::$packet_name => $id,)*
                }
            }

            pub fn state(&self) -> ConnectionState {
                match self {
                     $($type_enum_name::$packet_name => ConnectionState::$state,)*
                }
            }

            pub fn from_state_and_id_and_direction(state: ConnectionState, id: i32, bound_to: BoundTo) -> Option<$type_enum_name> {
                match (state, bound_to, id) {
                     $((ConnectionState::$state, BoundTo::$bound_to, $id) => Some($type_enum_name::$packet_name),)*
                     _ => None,
                }
            }
        }
    }
}
