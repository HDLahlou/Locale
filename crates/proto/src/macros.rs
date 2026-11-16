//! Macros that declare string enums with one place for each wire value.

/// Shared impls for `closed_enum!` and `open_enum!`.
macro_rules! enum_common {
    ($name:ident, $open:literal, [$($s:literal),+]) => {
        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }
        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = <::std::string::String as ::serde::Deserialize>::deserialize(d)?;
                s.parse().map_err(::serde::de::Error::custom)
            }
        }
        impl ::schemars::JsonSchema for $name {
            fn schema_name() -> ::std::borrow::Cow<'static, str> {
                stringify!($name).into()
            }
            fn json_schema(_: &mut ::schemars::SchemaGenerator) -> ::schemars::Schema {
                let description = if $open {
                    "Open enum: receivers map values they do not know to Unknown."
                } else {
                    "Closed enum: receivers reject values they do not know."
                };
                ::schemars::json_schema!({
                    "type": "string",
                    "enum": [$($s),+],
                    "description": description,
                })
            }
        }
    };
}

/// Declares a closed string enum: unknown values fail to parse.
macro_rules! closed_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident = $s:literal, )+
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $(#[$vmeta])* $variant, )+
        }

        impl $name {
            /// Every value, in contract order.
            pub const ALL: &'static [$name] = &[$($name::$variant,)+];
            /// Closed: unknown values are rejected.
            pub const OPEN: bool = false;

            /// The wire string.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $s, )+
                }
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::ParseError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $( $s => Ok($name::$variant), )+
                    _ => Err($crate::ParseError::UnknownValue {
                        kind: stringify!($name),
                        value: s.to_owned(),
                    }),
                }
            }
        }

        enum_common!($name, false, [$($s),+]);
    };
}

/// Declares an open string enum: unknown values parse to `Unknown`.
///
/// `Unknown` is receive-only. It serialises as `"unknown"` and must never be sent.
macro_rules! open_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident = $s:literal, )+
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $(#[$vmeta])* $variant, )+
            /// A value this version does not know. Receive-only; never sent.
            Unknown,
        }

        impl $name {
            /// Every known value, in contract order. `Unknown` is not included.
            pub const ALL: &'static [$name] = &[$($name::$variant,)+];
            /// Open: unknown values map to `Unknown`.
            pub const OPEN: bool = true;

            /// The wire string (`"unknown"` for `Unknown`).
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $s, )+
                    $name::Unknown => "unknown",
                }
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::ParseError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(match s {
                    $( $s => $name::$variant, )+
                    _ => $name::Unknown,
                })
            }
        }

        enum_common!($name, true, [$($s),+]);
    };
}
