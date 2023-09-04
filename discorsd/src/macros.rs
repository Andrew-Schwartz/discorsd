/// Like format but for cdn urls.
macro_rules! cdn {
    ($fmt:literal, $($args:tt)+) => {
        format!(concat!("https://cdn.discordapp.com/", $fmt), $($args)+)
    };
}

macro_rules! api {
    (VERSION) => { 10 };
    (@priv str $fmt:literal) => {
        std::concat!("https://discord.com/api/v", api!(VERSION), $fmt)
    };
    ($fmt:literal) => {
        api!(@priv str $fmt).to_string()
    };
    ($fmt:literal, $($args:tt)+) => {
        format!(api!(@priv str $fmt), $($args)+)
    };
}
pub const API_VERSION: u8 = api!(VERSION);

/// Derive `Serialize`, `Deserialize` for bitflags, (de)serializing as if this were an integer.
/// ```rust
/// use bitflags::bitflags;
/// bitflags! {
///     struct Flags: u8 {
///         const A = 1;
///         const B = 2;
///         const C = 4;
///     }
/// }
/// serde_bitflag!(Flags: u8);
/// ```
macro_rules! serde_bitflag {
    ($bitflag:ty: $repr:ty) => {
        impl serde::ser::Serialize for $bitflag {
            fn serialize<S: serde::ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                self.bits().serialize(s)
            }
        }

        impl<'de> serde::de::Deserialize<'de> for $bitflag {
            fn deserialize<D: serde::de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let bits = <$repr>::deserialize(d)?;
                Self::from_bits(bits)
                    .ok_or_else(|| serde::de::Error::custom(format!("Unexpected flags value: {}", bits)))
            }
        }
    };
}

/// Same as the `serde_repr` crate but doesn't upset clippy about not using `Self`, and is a normal
/// macro instead of a derive macro.
///
/// Derives `Debug, Clone, Copy, PartialEq, Eq, Hash`.
///
/// Only handles pub enums because that's all I need.
///
/// ```
/// serde_repr! {
///     pub enum Type: u8 {
///         TheTuple = 1,
///         TheUnit = 2,
///         TheStruct = 3,
///     }
/// }
/// ```
macro_rules! serde_repr {
    (
        $(#[$outer:meta])*
        pub enum $enum_name:ident: $repr:tt {
            $(
                $(#[$inner:meta])*
                $variant:ident = $num:literal
            ),* $(,)?
        }
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr($repr)]
        pub enum $enum_name {
            $(
                $(#[$inner])*
                $variant = $num
            ),*
        }

        impl ::serde::Serialize for $enum_name {
            fn serialize<S: ::serde::ser::Serializer>(&self, s: S) -> ::std::result::Result<S::Ok, S::Error> {
                (*self as $repr).serialize(s)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $enum_name {
            fn deserialize<D: ::serde::de::Deserializer<'de>>(d: D) -> ::std::result::Result<Self, D::Error> {
                use serde::de::{Error, Unexpected};

                match <$repr>::deserialize(d)? {
                    $(
                        $num => ::std::result::Result::Ok(Self::$variant),
                    )*
                    other => ::std::result::Result::Err(D::Error::invalid_value(
                        Unexpected::Unsigned(other as _),
                        &concat!(
                            "one of: ",
                            $(
                                stringify!($variant = $num),
                                ", ",
                            )*
                        )
                    ))
                }
            }
        }
    };
}

/// Macro to implement Deserialize and Serialize for an enum with variants differentiated by any
/// field in the json.
///
/// For example,
/// ```json
/// {
///   "type": 1,
///   "name": "Cool Name"
/// }
/// ```
/// vs
/// ```json
/// {
///   "type": 2,
///   "number": 42
/// }
/// ```
/// can be (de)serialized with
/// ```
/// serde_num_tag! {
///     pub enum MyData = "type": u8 as MyDataType {
///         (1) => StringVariant(String),
///         (2) => Number {
///             /// documentation is allowed, as are serde meta tags, with the following syntax:
///             #serde = rename = "number"
///             n: usize
///         },
///     }
/// }
// todo error if any tag_value's repeat
macro_rules! serde_num_tag {
    // some helper macros

    // `type` as `new_enum`
    (
        make_serde_repr
        [ ]
        $enum_name:ident, $tag_type:ty,
        $($var_name:ident, [$($tag_value:tt)+])+
    ) => {
        /* no enum for types, and just give back the u8 or w/e */
        impl $enum_name {
            #[allow(dead_code)]
            pub const fn variant_type(&self) -> $tag_type {
                match self {
                    $(
                        Self::$var_name { .. } => $($tag_value)+,
                    )+
                }
            }
        }
    };
    (
        make_serde_repr
        [ $repr_name:ident ]
        $enum_name:ident, $tag_type:ty,
        $($var_name:ident, [$($tag_value:tt)+])+
    ) => {
        serde_repr! {
            pub enum $repr_name: $tag_type {
                $(
                    $var_name = $($tag_value)+,
                )+
            }
        }
        impl $enum_name {
            #[allow(dead_code)]
            pub const fn variant_type(&self) -> $repr_name {
                match self {
                    $(
                        Self::$var_name { .. } => $repr_name::$var_name,
                    )+
                }
            }
        }
    };
    // allow to skip deserializing
    (
        deser_impl
        [ just Deserialize ]
        $impl_braced:block
    ) => {
        $impl_braced
    };
    (
        deser_impl
        [ just Serialize ]
        $impl_braced:block
    ) => {
        { /* skip */ }
    };
    (
        deser_impl
        [ ]
        $impl_braced:block
    ) => {
        $impl_braced
    };
    // allow to skip serializing
    (
        ser_impl
        [ just Serialize ]
        $impl_braced:block
    ) => {
        $impl_braced
    };
    (
        ser_impl
        [ just Deserialize ]
        $impl_braced:block
    ) => {
        { /* skip */ }
    };
    (
        ser_impl
        [ ]
        $impl_braced:block
    ) => {
        $impl_braced
    };
    (ser_match_pat_tuple [ $ignore:ty ] $t:pat) => {
        $t
    };
    (
        ser_shim
        [ $rename:literal ]
        [ $tag_name:literal, $tag_type:ty ]
    ) => {
        #[derive(::serde_derive::Serialize)]
        struct Shim<'t, T> {
            #[serde(rename = $tag_name)]
            variant: $tag_type,
            #[serde(rename = $rename)]
            t: &'t T,
        }
    };
    (
        ser_shim
        [ ]
        [ $tag_name:literal, $tag_type:ty ]
    ) => {
        #[derive(::serde_derive::Serialize)]
        struct Shim<'t, T> {
            #[serde(rename = $tag_name)]
            variant: $tag_type,
            #[serde(flatten)]
            t: &'t T,
        }
    };
    // unit
    (
        ser_match_arm
        [ ]
        [ ]
        $tag_value:expr;
        $s:expr
    ) => {
        UnitShim { variant: $tag_value }.serialize($s)
    };
    // struct
    (
        ser_match_arm
        [ ]
        [ $($field_name:ident, $field_type:ty $(,$serde_meta:tt)* ;)+ ; $tag_name:literal, $tag_type:ty ]
        $tag_value:expr;
        $s:expr
    ) => {
        #[derive(::serde_derive::Serialize)]
        struct Shim<'a> {
            #[serde(rename = $tag_name)]
            variant: $tag_type,
            $(
                $(#[serde($serde_meta)])*
                $field_name: &'a $field_type,
            )+
        }
        Shim { variant: $tag_value, $($field_name,)+ }.serialize($s)
    };
    // tuple
    (
        ser_match_arm
        [ $ignore_inner:ty; $t:expr ]
        [ ]
        $tag_value:expr;
        $s:expr
    ) => {
        Shim { variant: $tag_value, t: $t }.serialize($s)
    };
    // main macro
    (
        $(just $skip_ser_or_de:tt =>)?
        $(#[$enum_meta:meta])*
        pub enum $enum_name:ident = $tag_name:literal $(alias $($alias:literal),+ $(,)?)? :
                 $tag_type:ty $(as $repr_name:ident)? $(, inner = $rename:literal)? {
            $(
                // todo distinguish the docs from the other meta stuff and copy those into the repr
                $(#[$variant_meta:meta])*
                ( $($tag_value:tt)+ ) =
                $var_name:ident
                // parenthesized tuple
                $((
                        $(#[$tuple_field_meta:meta])*
                        $tuple_field_type:ty
                ))?
                // braced struct
                $({
                    $(
                        $(#[$struct_field_meta:meta])*
                        $(#serde = $struct_field_serde:meta)*
                        $struct_field_name:ident: $struct_field_type:ty
                    ),+ $(,)?
                })?
            ),+ $(,)?
        }
    ) => {
        $(#[$enum_meta])*
        pub enum $enum_name {
            $(
                $(#[$variant_meta])*
                $var_name
                // parens
                $(
                    (
                        $(#[$tuple_field_meta])*
                        $tuple_field_type
                    )
                )?
                // braces
                $({
                    $(
                        $(#[$struct_field_meta])*
                        $struct_field_name: $struct_field_type,
                    )+
                })?,
            )+
        }

        // make a serde_repr enum for the values of this
        serde_num_tag! {
            make_serde_repr
            [ $($repr_name)? ]
            $enum_name, $tag_type,
            $($var_name, [$($tag_value)+])+
        }

        const _: () = serde_num_tag! {
            deser_impl
            [$(just $skip_ser_or_de)?]
            {
                impl<'de> ::serde::Deserialize<'de> for $enum_name {
                    fn deserialize<D: ::serde::Deserializer<'de>>(d: D) -> ::std::result::Result<Self, D::Error> {
                        let value = <::serde_json::Value>::deserialize(d)?;
                        let variant_value = value.get($tag_name)
                            $($(
                                .or_else(|| value.get($alias))
                            )+)?
                            .ok_or_else(|| ::serde::de::Error::missing_field($tag_name))?;
                        let variant = ::serde_json::from_value(variant_value.clone())
                            .map_err(::serde::de::Error::custom)?;
                        let this = match variant {
                            $(
                                $($tag_value)+ => {
                                    // extra stuff for handling braced variants
                                    $(
                                        #[derive(::serde_derive::Deserialize)]
                                        struct Shim {
                                            $(
                                                $(#[serde($struct_field_serde)])*
                                                $(#[$struct_field_meta])*
                                                $struct_field_name: $struct_field_type,
                                            )+
                                        }
                                        let Shim { $($struct_field_name,)+ } = ::serde_json::from_value(value).map_err(::serde::de::Error::custom)?;
                                    )?
                                    Self::$var_name
                                    $( (::serde_json::from_value::<$tuple_field_type>(value).map_err(::serde::de::Error::custom)?) )?
                                    $( { $($struct_field_name,)+ } )?
                                }
                            )+
                            #[allow(unreachable_patterns)]
                            bad => return Err(::serde::de::Error::unknown_variant(
                                &::serde_json::to_string(&bad).unwrap(),
                                &[
                                    $(
                                        stringify!($($tag_value)+),
                                    )+
                                ]
                            )),
                        };
                        Ok(this)
                    }
                }
            }
        };

        const _: () = serde_num_tag! {
            ser_impl
            [$(just $skip_ser_or_de)?]
            {
                impl ::serde::Serialize for $enum_name {
                    fn serialize<S: ::serde::Serializer>(&self, s: S) -> ::std::result::Result<S::Ok, S::Error> {
                        #[derive(::serde_derive::Serialize)]
                        struct UnitShim {
                            #[serde(rename = $tag_name)]
                            variant: $tag_type,
                        }
                        serde_num_tag! {
                            ser_shim
                            [ $($rename)? ]
                            [ $tag_name, $tag_type ]
                        }

                        match self {
                            $(
                                Self::$var_name
                                // parens
                                $(
                                    (serde_num_tag!(ser_match_pat_tuple [$tuple_field_type] t))
                                )?
                                // braces
                                $(
                                    {
                                        $(
                                            $struct_field_name,
                                        )+
                                    }
                                )?
                                => {
                                    serde_num_tag! {
                                        ser_match_arm
                                        [ $($tuple_field_type; t)? ]
                                        [ $( $($struct_field_name, $struct_field_type $(, $struct_field_serde)* ;)+ ; $tag_name, $tag_type)? ]
                                        $($tag_value)+;
                                        s
                                    }
                                }
                            )+
                        }
                    }
                }
            }
        };
    };
}

#[cfg(test)]
mod tag_by_num {
    use std::fmt::Debug;

    use serde_derive::{Deserialize, Serialize};
    use serde::de::DeserializeOwned;

    use crate::model::ids::*;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Text {
        #[serde(rename = "channel")]
        id: ChannelId,
        name: String,
    }

    fn test<T: DeserializeOwned + Serialize + Debug>(json: &'static str) -> serde_json::Result<()> {
        let result: T = serde_json::from_str(json)?;
        println!("result = {:?}", result);
        let back = serde_json::to_string_pretty(&result)?;
        assert_eq!(json, back);
        Ok(())
    }

    fn test_fail_deserialize<T: DeserializeOwned + Debug>(json: &'static str, message: &str) {
        let error = serde_json::from_str::<T>(json)
            .expect_err(message);
        println!("error = {:?}", error);
    }

    serde_num_tag! {
        #[derive(Debug)]
        pub enum TestEnum = "type": u8 {
            /// docs
            (2) = Unit,
            (1) = Tuple(Text),
            (3) = Struct {
                // todo add some sort of support for serde stuff
                /// docs
                #serde = rename = "user"
                id: UserId,
                name: String,
            },
        }
    }

    #[test]
    fn unit() -> serde_json::Result<()> {
        const JSON: &str = r#"{
  "type": 2
}"#;
        test::<TestEnum>(JSON)
    }

    #[test]
    fn tuple() -> serde_json::Result<()> {
        const JSON: &str = r#"{
  "type": 1,
  "channel": "1123",
  "name": "TestName"
}"#;
        test::<TestEnum>(JSON)
    }

    #[test]
    fn r#struct() -> serde_json::Result<()> {
        const JSON: &str = r#"{
  "type": 3,
  "user": "1123",
  "name": "TestName"
}"#;
        test::<TestEnum>(JSON)
    }

    #[test]
    fn mismatched_type() {
        const JSON: &str = r#"{
  "type": 3,
  "channel": "1123",
  "name": "TestName"
}"#;
        test_fail_deserialize::<TestEnum>(JSON, "Try to deserialize type 3 with type 1 data");
    }

    #[test]
    fn nonexistent_type() {
        const JSON: &str = r#"{
  "type": 4,
  "id": "1123",
  "name": "TestName"
}"#;
        test_fail_deserialize::<TestEnum>(JSON, "Type 4 doesn't exist");
    }

    serde_repr! {
        pub enum Type: u8 {
            TheTuple = 1,
            TheUnit = 2,
            TheStruct = 3,
        }
    }

    serde_num_tag! {
        #[derive(Debug)]
        pub enum TestEnumTyped = "type": Type {
            /// docs
            (Type::TheUnit) = Unit,
            (Type::TheTuple) = Tuple(Text),
            (Type::TheStruct) = Struct {
                /// docs
                #serde = rename = "user"
                id: ChannelId,
                name: String,
            },
        }
    }


    #[test]
    fn unit_typed() -> serde_json::Result<()> {
        const JSON: &str = r#"{
  "type": 2
}"#;
        test::<TestEnumTyped>(JSON)
    }

    #[test]
    fn tuple_typed() -> serde_json::Result<()> {
        const JSON: &str = r#"{
  "type": 1,
  "channel": "1123",
  "name": "TestName"
}"#;
        test::<TestEnumTyped>(JSON)
    }

    #[test]
    fn struct_typed() -> serde_json::Result<()> {
        const JSON: &str = r#"{
  "type": 3,
  "user": "1123",
  "name": "TestName"
}"#;
        test::<TestEnumTyped>(JSON)
    }

    #[test]
    fn mismatched_type_typed() {
        const JSON: &str = r#"{
  "type": 3,
  "channel": "1123",
  "name": "TestName"
}"#;
        test_fail_deserialize::<TestEnumTyped>(JSON, "Try to deserialize type 3 with type 1 data");
    }

    #[test]
    fn nonexistent_type_typed() {
        const JSON: &str = r#"{
  "type": 4,
  "id": "1123",
  "name": "TestName"
}"#;
        test_fail_deserialize::<TestEnumTyped>(JSON, "Type 4 doesn't exist");
    }

    #[test]
    fn get_variant_typed() {
        const JSON: &str = r#"{
  "type": 3,
  "user": "1123",
  "name": "TestName"
}"#;
        let blah: TestEnumTyped = serde_json::from_str(JSON).unwrap();
        assert_eq!(Type::TheStruct, blah.variant_type())
    }
}
