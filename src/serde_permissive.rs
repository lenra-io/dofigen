#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{
    de::{self, value::Error, Error as DeError, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::{
    fmt::{self},
    str::FromStr,
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum PermissiveStruct<T>
where
    T: FromStr<Err = Error>,
{
    Int(isize),
    Uint(usize),
    String(String),
    Struct(T),
}

pub fn deserialize_optional_one_or_many<'de, D, T>(
    deserializer: D,
) -> Result<Option<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct OptionalOneOrManyVisitor<T>(Option<T>);

    impl<'de, T> Visitor<'de> for OptionalOneOrManyVisitor<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Option<Vec<T>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an optional str, map or seq")
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            let value: Vec<T> = deserialize_one_or_many(deserializer)?;
            Ok(Some(value))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(None)
        }
    }

    let visitor: OptionalOneOrManyVisitor<T> = OptionalOneOrManyVisitor(None);

    deserializer.deserialize_option(visitor)
}

pub fn deserialize_one_or_many<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct OneOrManyVisitor<T>(Option<T>);

    impl<'de, T> Visitor<'de> for OneOrManyVisitor<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a number, a string, a map or a seq")
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
            let value: T = Deserialize::deserialize(de::value::StrDeserializer::new(v))?;
            Ok(vec![value])
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let value: T = Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
            Ok(vec![value])
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

#[cfg(test)]
mod test {
    use super::*;

    mod deserialize_one_or_many {
        use super::*;

        #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_one_or_many")]
            pub one_or_many: Vec<String>,
        }

        #[test]
        fn one() {
            let ret: TestStruct = serde_yaml::from_str("one_or_many: test").unwrap();
            assert_eq!(
                ret,
                TestStruct {
                    one_or_many: vec!["test".to_string()]
                }
            )
        }

        #[test]
        fn many() {
            let ret: TestStruct = serde_yaml::from_str("one_or_many: [test]").unwrap();
            assert_eq!(
                ret,
                TestStruct {
                    one_or_many: vec!["test".to_string()]
                }
            )
        }
    }

    mod deserialize_optional_one_or_many {
        use super::*;

        #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
        struct TestStruct {
            pub test: Option<String>,
            #[serde(deserialize_with = "deserialize_optional_one_or_many", default)]
            pub one_or_many: Option<Vec<String>>,
        }

        #[test]
        fn one() {
            let ret: TestStruct = serde_yaml::from_str("one_or_many: test").unwrap();
            assert_eq!(
                ret,
                TestStruct {
                    test: None,
                    one_or_many: Some(vec!["test".to_string()])
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
                    one_or_many: Some(vec!["test".to_string()])
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
                    test: Some("test".to_string()),
                    one_or_many: None
                }
            )
        }
    }
}
