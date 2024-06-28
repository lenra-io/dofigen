use std::{
    fmt::{self},
    marker::PhantomData,
    str::FromStr,
};

use serde::{
    de::{self, Error as DeError, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{Copy, CopyResources, Error, ImageName, ImageVersion, OneOrMany};

pub fn string_or_struct<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Error>,
    D: Deserializer<'de>,
{
    struct StringOrStruct<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = Error>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: DeError,
        {
            Ok(FromStr::from_str(value).unwrap())
        }

        fn visit_map<M>(self, map: M) -> Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

// pub fn single_or_vec<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
// where
//     T: Deserializer<'de>,
//     D: Deserializer<'de>,
// {
//     struct SingleOrVec<T>(PhantomData<fn() -> Vec<T>>);

//     impl<'de, T> Visitor<'de> for SingleOrVec<T>
//     where
//         T: Deserialize<'de>,
//     {
//         type Value = Vec<T>;

//         fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//             formatter.write_str("string or array")
//         }

//         fn visit_str<E>(self, value: &str) -> Result<Vec<T>, E>
//         where
//             E: DeError,
//         {
//             Ok(vec![])
//         }

//         fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
//         where
//             A: MapAccess<'de>,
//         {
//             let value: T = Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
//             Ok(vec![value])
//         }

//         fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
//         where
//             A: de::SeqAccess<'de>,
//         {
//             Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
//         }
//     }

//     deserializer.deserialize_any(SingleOrVec(PhantomData))
// }

impl<'de> Deserialize<'de> for ImageName {
    fn deserialize<D>(deserializer: D) -> Result<ImageName, D::Error>
    where
        D: Deserializer<'de>,
    {
        string_or_struct(deserializer)
    }
}

struct CopyVisitor;

impl<'de> Visitor<'de> for CopyVisitor {
    type Value = CopyResources;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an integer between -2^31 and 2^31")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: DeError, {
        println!("visit_str: {}", v);
        Ok(CopyResources::Copy(Copy {
            paths: OneOrMany::One(v.to_string()),
            ..Default::default()
        }))
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        println!("visit_map");
        Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
    }
}

impl<'de> Deserialize<'de> for CopyResources {
    fn deserialize<D>(deserializer: D) -> Result<CopyResources, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(CopyVisitor)
    }
}

impl From<String> for ImageName {
    fn from(s: String) -> Self {
        print!("s: {}", s);
        let mut parts = s.splitn(2, '/');
        let host = parts.next().map(|s| s.to_string());
        let mut parts = parts.next().unwrap().splitn(2, ':');
        let path = parts.next().unwrap().to_string();
        let mut parts = parts.next().unwrap().splitn(2, '@');
        let version = parts.next().map(|s| s.to_string());
        ImageName {
            host,
            port: None,
            path,
            version: version.map(|v| ImageVersion::Tag(v)),
        }
    }
}

impl FromStr for ImageName {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        print!("s: {}", s);
        let host = None;
        let path = s.into();
        Ok(ImageName {
            host,
            port: None,
            path,
            version: None,
        })
    }
}
