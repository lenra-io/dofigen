#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{
    de::{self, DeserializeOwned, Error as DeError, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, ops::Deref, str::FromStr};
use struct_patch::*;

use crate::dofigen_struct::*;

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct ParsableStruct<T>(T)
where
    T: FromStr + Sized;

impl<T: FromStr + Sized> ParsableStruct<T> {
    pub fn new(value: T) -> Self {
        ParsableStruct(value)
    }
}

impl<T: FromStr + Sized> Deref for ParsableStruct<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<'de, T> Deserialize<'de> for ParsableStruct<T>
where
    T: FromStr + Sized + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<ParsableStruct<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_permissive_struct(deserializer).map(ParsableStruct::new)
    }
}

impl<T: FromStr + Sized> From<T> for ParsableStruct<T> {
    fn from(value: T) -> Self {
        ParsableStruct::new(value)
    }
}

fn deserialize_permissive_struct<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + Sized + Deserialize<'de>,
{
    struct PermissiveStructVisitor<T>(Option<T>);

    impl<'de, T> Visitor<'de> for PermissiveStructVisitor<T>
    where
        T: Deserialize<'de> + FromStr,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a number, a string or a map")
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            self.visit_str(v.to_string().as_str())
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            self.visit_str(v.to_string().as_str())
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            // TODO: improve error management
            v.parse()
                .map_err(|_| E::custom("Error while parsing a permissive struct"))
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    let visitor: PermissiveStructVisitor<T> = PermissiveStructVisitor(None);

    deserializer.deserialize_any(visitor)
}

pub fn deserialize_permissive_type<'de, T2, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + From<T2>,
{
    // let tmp:T2 = deserializer.deserialize_any()?;
    // Ok(T::from(tmp))
    println!("deserialize_permissive_type");
    todo!()
}

macro_rules! impl_parsable_patch {
    ($struct:ty, $patch:ty) => {
        impl Patch<ParsableStruct<$patch>> for $struct {
            fn apply(&mut self, patch: ParsableStruct<$patch>) {
                self.apply(patch.0);
            }

            fn into_patch(self) -> ParsableStruct<$patch> {
                ParsableStruct(self.into_patch())
            }

            fn into_patch_by_diff(self, previous_struct: Self) -> ParsableStruct<$patch> {
                ParsableStruct(self.into_patch_by_diff(previous_struct))
            }

            fn new_empty_patch() -> ParsableStruct<$patch> {
                ParsableStruct(Self::new_empty_patch())
            }
        }

        impl From<ParsableStruct<$patch>> for $struct {
            fn from(value: ParsableStruct<$patch>) -> Self {
                value.0.into()
            }
        }
    };
}

impl_parsable_patch!(ImageName, ImageNamePatch);
impl_parsable_patch!(User, UserPatch);
impl_parsable_patch!(CopyResource, CopyResourcePatch);
impl_parsable_patch!(Copy, CopyPatch);
impl_parsable_patch!(Add, AddPatch);
impl_parsable_patch!(AddGitRepo, AddGitRepoPatch);
impl_parsable_patch!(Port, PortPatch);

#[cfg(test)]
mod test {
    use super::*;
    use crate::deserialize_struct::OneOrManyVec;
    use pretty_assertions_sorted::assert_eq_sorted;

    mod deserialize_one_or_many {
        use super::*;

        #[derive(Deserialize, Debug, Clone, PartialEq, Default)]
        struct TestStruct {
            pub one_or_many: OneOrManyVec<String>,
        }

        #[test]
        fn one() {
            let ret: TestStruct = serde_yaml::from_str("one_or_many: test").unwrap();
            assert_eq_sorted!(
                ret,
                TestStruct {
                    one_or_many: OneOrManyVec::new(vec!["test".into()])
                }
            )
        }

        #[test]
        fn many() {
            let ret: TestStruct = serde_yaml::from_str("one_or_many: [test]").unwrap();
            assert_eq_sorted!(
                ret,
                TestStruct {
                    one_or_many: OneOrManyVec::new(vec!["test".into()])
                }
            )
        }
    }

    mod deserialize_optional_one_or_many {
        use super::*;

        #[derive(Deserialize, Debug, Clone, PartialEq, Default)]
        struct TestStruct {
            pub test: Option<String>,
            pub one_or_many: Option<OneOrManyVec<String>>,
        }

        #[test]
        fn one() {
            let ret: TestStruct = serde_yaml::from_str("one_or_many: test").unwrap();
            assert_eq_sorted!(
                ret,
                TestStruct {
                    test: None,
                    one_or_many: Some(vec!["test".into()].into())
                }
            )
        }

        #[test]
        fn many() {
            let ret: TestStruct = serde_yaml::from_str("one_or_many: [test]").unwrap();
            assert_eq_sorted!(
                ret,
                TestStruct {
                    test: None,
                    one_or_many: Some(vec!["test".into()].into())
                }
            )
        }

        #[test]
        fn null() {
            let ret: TestStruct = serde_yaml::from_str("one_or_many: null").unwrap();
            assert_eq_sorted!(
                ret,
                TestStruct {
                    test: None,
                    one_or_many: None
                }
            )
        }

        #[test]
        fn absent() {
            let ret: TestStruct = serde_yaml::from_str("test: test").unwrap();
            assert_eq_sorted!(
                ret,
                TestStruct {
                    test: Some("test".into()),
                    one_or_many: None
                }
            )
        }
    }
}
