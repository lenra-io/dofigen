use std::{
    fmt::{self},
    marker::PhantomData,
    str::FromStr,
};

use serde::{
    de::{self, Error as DeError, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

use crate::{Copy, CopyResources, Error, ImageName, ImageVersion};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum StringOrStruct<T>
where
    T: FromStr<Err = Error>,
{
    String(String),
    Struct(T),
}

macro_rules! impl_Stage {
    (for $($t:ty),+) => {
        $(impl From<StringOrStruct<$t>> for $t {
            fn from(s: StringOrStruct<$t>) -> Self {
                match s {
                    StringOrStruct::String(s) => s.parse().unwrap(),
                    StringOrStruct::Struct(s) => s,
                }
            }
        })*
    }
}

impl_Stage!(for ImageName, CopyResources, Copy);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Vec(Vec<T>),
}

impl<T> From<OneOrMany<T>> for Vec<T> {
    fn from(v: OneOrMany<T>) -> Self {
        match v {
            OneOrMany::One(v) => vec![v],
            OneOrMany::Vec(v) => v,
        }
    }
}

impl Default for ImageVersion {
    fn default() -> Self {
        ImageVersion::Tag("latest".into())
    }
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
            println!("optional one or many from some");
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
            formatter.write_str("a str, a map or a seq")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            println!("one or many from str");
            let value: T = Deserialize::deserialize(de::value::StrDeserializer::new(v))?;
            Ok(vec![value])
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            println!("one or many from map");
            let value: T = Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
            Ok(vec![value])
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            println!("one or many from seq");
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }
    }

    let visitor: OneOrManyVisitor<T> = OneOrManyVisitor(None);

    deserializer.deserialize_any(visitor)
}

impl FromStr for CopyResources {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        print!("CopyResources: {}", s);
        Ok(CopyResources::Copy(s.parse().unwrap()))
    }
}

impl FromStr for Copy {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        print!("Copy: {}", s);
        Ok(Copy {
            paths: vec![s.into()],
            target: None,
            chown: None,
            chmod: None,
            exclude: None,
            link: None,
            parents: None,
            from: None,
        })
    }
}

impl FromStr for ImageName {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        print!("ImageName: {}", s);
        let host = None;
        let path = s.into();
        Ok(ImageName {
            host,
            port: None,
            path,
            version: ImageVersion::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_name_from_str() {
        let input = "example/image";
        let result = ImageName::from_str(input).unwrap();
        assert!(result.host.is_none());
        assert_eq!(result.path, "example/image");
        assert!(result.port.is_none());
        assert_eq!(result.version, ImageVersion::Tag("latest".into()));
    }

    #[test]
    fn image_name_from_str_with_host() {
        let input = "example/image:tag";
        let result = ImageName::from_str(input).unwrap();
        assert!(result.host.is_none());
        assert_eq!(result.path, "example/image");
        assert!(result.port.is_none());
        assert_eq!(result.version, ImageVersion::Tag("tag".into()));
    }

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
