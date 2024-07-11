#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{
    de::{self, Error as DeError, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::{fmt, ops::Deref, str::FromStr};

#[derive(Serialize, Debug, Clone, PartialEq, Default)]
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

#[derive(Serialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct OneOrManyVec<T>(Vec<T>)
where
    T: Sized;

impl<T: Sized + Clone> OneOrManyVec<T> {
    pub fn new(value: Vec<T>) -> Self {
        OneOrManyVec(value)
    }

    // pub fn deref(&self) -> &Vec<T> {
    //     &self.0
    // }

    // pub fn to_vec(&self) -> Vec<T> {
    //     self.deref().to_vec()
    // }

    // pub fn is_empty(&self) -> bool {
    //     self.deref().is_empty()
    // }
}

impl<T: Sized + Clone> Deref for OneOrManyVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T: Sized + Clone> From<Vec<T>> for OneOrManyVec<T> {
    fn from(value: Vec<T>) -> Self {
        OneOrManyVec::new(value)
    }
}

impl<'de, T> Deserialize<'de> for OneOrManyVec<T>
where
    T: Sized + Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<OneOrManyVec<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_one_or_many_vec(deserializer).map(OneOrManyVec::new)
    }
}

fn deserialize_one_or_many_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct OneOrManyVisitor<T>(Option<T>);

    fn map_vec<T>(value: T) -> Vec<T> {
        vec![value]
    }

    impl<'de, T> Visitor<'de> for OneOrManyVisitor<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("any type")
        }

        fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Deserialize::deserialize(de::value::I8Deserializer::new(v)).map(map_vec)
        }

        fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Deserialize::deserialize(de::value::I16Deserializer::new(v)).map(map_vec)
        }

        fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Deserialize::deserialize(de::value::I32Deserializer::new(v)).map(map_vec)
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Deserialize::deserialize(de::value::I64Deserializer::new(v)).map(map_vec)
        }

        fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Deserialize::deserialize(de::value::I128Deserializer::new(v)).map(map_vec)
        }

        fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Deserialize::deserialize(de::value::U8Deserializer::new(v)).map(map_vec)
        }

        fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Deserialize::deserialize(de::value::U16Deserializer::new(v)).map(map_vec)
        }

        fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Deserialize::deserialize(de::value::U32Deserializer::new(v)).map(map_vec)
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Deserialize::deserialize(de::value::U64Deserializer::new(v)).map(map_vec)
        }

        fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Deserialize::deserialize(de::value::U128Deserializer::new(v)).map(map_vec)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Deserialize::deserialize(de::value::StrDeserializer::new(v)).map(map_vec)
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map)).map(map_vec)
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }
    }

    let visitor: OneOrManyVisitor<T> = OneOrManyVisitor(None);

    deserializer.deserialize_any(visitor)
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

#[cfg(test)]
mod test {
    use super::*;

    mod deserialize_one_or_many {
        use super::*;

        #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
        struct TestStruct {
            pub one_or_many: OneOrManyVec<String>,
        }

        #[test]
        fn one() {
            let ret: TestStruct = serde_yaml::from_str("one_or_many: test").unwrap();
            assert_eq!(
                ret,
                TestStruct {
                    one_or_many: OneOrManyVec::new(vec!["test".into()])
                }
            )
        }

        #[test]
        fn many() {
            let ret: TestStruct = serde_yaml::from_str("one_or_many: [test]").unwrap();
            assert_eq!(
                ret,
                TestStruct {
                    one_or_many: OneOrManyVec::new(vec!["test".into()])
                }
            )
        }
    }

    mod deserialize_optional_one_or_many {
        use super::*;

        #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
        struct TestStruct {
            pub test: Option<String>,
            pub one_or_many: Option<OneOrManyVec<String>>,
        }

        #[test]
        fn one() {
            let ret: TestStruct = serde_yaml::from_str("one_or_many: test").unwrap();
            assert_eq!(
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
            assert_eq!(
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
            assert_eq!(
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
            assert_eq!(
                ret,
                TestStruct {
                    test: Some("test".into()),
                    one_or_many: None
                }
            )
        }
    }
}
