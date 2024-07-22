use crate::dofigen_struct::*;
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{
    de::{self, Error as DeError, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_yaml::Value;
use std::{fmt, ops::Deref, str::FromStr};

pub trait Merge: Clone {
    fn merge(&self, other: Self) -> Self;
}

#[derive(Serialize, Debug, PartialEq)]
// #[serde(untagged)]
pub enum OptionalField<T> {
    Missing,
    Null,
    Present(T),
}

impl<T: Sized + Clone> OptionalField<T> {
    pub fn as_ref(&self) -> OptionalField<&T> {
        match self {
            OptionalField::Present(v) => OptionalField::Present(v),
            OptionalField::Missing => OptionalField::Missing,
            OptionalField::Null => OptionalField::Null,
        }
    }

    pub fn expect(self, msg: &str) -> T {
        match self {
            OptionalField::Present(v) => v,
            _ => panic!("{}", msg),
        }
    }

    pub fn unwrap(self) -> T {
        self.expect("Called `Patch::unwrap()` on a `Missing` value")
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            OptionalField::Present(v) => v,
            _ => default,
        }
    }

    pub fn or(self, other: OptionalField<T>) -> OptionalField<T> {
        match self {
            OptionalField::Present(_) => self,
            _ => other,
        }
    }

    pub fn ok_or<E>(self, err: E) -> Result<T, E> {
        match self {
            OptionalField::Present(v) => Ok(v),
            _ => Err(err),
        }
    }

    pub fn to_option(self) -> Option<T> {
        match self {
            OptionalField::Present(v) => Some(v),
            _ => None,
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> OptionalField<U> {
        match self {
            OptionalField::Present(v) => OptionalField::Present(f(v)),
            OptionalField::Missing => OptionalField::Missing,
            OptionalField::Null => OptionalField::Null,
        }
    }

    pub fn is_present(&self) -> bool {
        matches!(self, OptionalField::Present(_))
    }

    pub fn is_missing(&self) -> bool {
        matches!(self, OptionalField::Missing)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, OptionalField::Null)
    }
}

impl<T> Default for OptionalField<T> {
    fn default() -> Self {
        OptionalField::Missing
    }
}

impl<T> Clone for OptionalField<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        match self {
            OptionalField::Missing => OptionalField::Missing,
            OptionalField::Null => OptionalField::Null,
            OptionalField::Present(v) => OptionalField::Present(v.clone()),
        }
    }
}

impl<T> From<Option<T>> for OptionalField<T> {
    fn from(opt: Option<T>) -> OptionalField<T> {
        match opt {
            Some(v) => OptionalField::Present(v),
            None => OptionalField::Null,
        }
    }
}

impl<'de, T> Deserialize<'de> for OptionalField<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::deserialize(deserializer).map(Into::into)
    }
}

impl<T: Merge> Merge for OptionalField<T> {
    fn merge(&self, other: Self) -> Self {
        match (self, other) {
            (OptionalField::Present(a), OptionalField::Present(b)) => OptionalField::Present(a.merge(b)),
            (_, OptionalField::Missing) => self.clone(),
            (_, other) => other,
        }
    }
}

impl<T: Merge> Merge for Box<T> {
    fn merge(&self, other: Self) -> Self {
        Box::new(self.as_ref().merge(other.deref().clone()))
    }
}

impl Merge for OptionalField<String> {
    fn merge(&self, other: Self) -> Self {
        match (self, other) {
            (OptionalField::Missing, other) => other,
            (_, OptionalField::Missing) => self.clone(),
            (_, other) => other,
        }
    }
}

impl Merge for OptionalField<u16> {
    fn merge(&self, other: Self) -> Self {
        match (self, other) {
            (OptionalField::Missing, other) => other,
            (_, OptionalField::Missing) => self.clone(),
            (_, other) => other,
        }
    }
}

// TODO: to merge properly, we need to save the original keys present in the file structure.
// Maybe defining a box that saves the original keys and then deserialize the struct.
// This would permit to remove the Option<> from the required fields.

// Another solution would be to have a different structure for deserializing and the final one.
// This would also remove the permissive structs in the final one.

// impl Merge for Value {
//     fn merge(&self, other: Self) -> Self {
//         match (self, other) {
//             (Value::Mapping(a), Value::Mapping(b)) => {
//                 let mut merged = a.clone();
//                 for (key, value) in b {
//                     merged.insert(key.clone(), a.get(key).unwrap_or(&Value::Null).merge(value));
//                 }
//                 Value::Mapping(merged)
//             }
//             (_, Value::Null) => self.clone(),
//             (_, other) => other,
//         }
//     }
// }

// macro_rules! impl_Merge {
// 	(for $($t:ty),+) => {
// 			$(impl Merge for $t {
// 				fn merge(&self, other: Self) -> Self {
// 						let a = serde_yaml::to_value(self).unwrap();
// 						let b = serde_yaml::to_value(other).unwrap();
// 						serde_yaml::from_value(a.merge(b)).unwrap()
// 				}
// 			})*
// 	}
// }

// impl_Merge!(for Builder, Image, Root);

impl Merge for Image {
    fn merge(&self, other: Self) -> Self {
        Image {
            from: self.from.merge(other.from),
            ..Default::default()
        }
    }
}

impl Merge for ImageName {
    fn merge(&self, other: Self) -> Self {
        ImageName {
            host: self.host.merge(other.host),
            path: self.path.merge(other.path),
            version: self.version.merge(other.version),
            port: self.port.merge(other.port),
        }
    }
}

impl Merge for ImageVersion {
    fn merge(&self, other: Self) -> Self {
        other
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod optional_field {
        use super::*;

        #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
        struct TestStruct {
            pub test: OptionalField<String>,
        }

        #[test]
        fn deserialize_present_field() {
            let ret: TestStruct = serde_yaml::from_str("test: test").unwrap();
            assert_eq!(
                ret,
                TestStruct {
                    test: OptionalField::Present("test".into())
                }
            )
        }

        #[test]
        fn deserialize_missing_field() {
            let ret: TestStruct = serde_yaml::from_str("").unwrap();
            assert_eq!(
                ret,
                TestStruct {
                    test: OptionalField::Missing
                }
            )
        }

        #[test]
        fn deserialize_null_field() {
            let ret: TestStruct = serde_yaml::from_str("test: null").unwrap();
            assert_eq!(
                ret,
                TestStruct {
                    test: OptionalField::Null
                }
            )
        }
    }

    mod image {
        use crate::ImageName;

        use super::*;

        #[test]
        fn extends_from_tag() {
            let base = Image {
                from: OptionalField::Present(
                    ImageName {
                        path: OptionalField::Present("ubuntu".into()),
                        ..Default::default()
                    }
                    .into(),
                ),
                ..Default::default()
            };

            let extended = Image {
                from: OptionalField::Present(
                    ImageName {
                        version: OptionalField::Present(crate::ImageVersion::Tag("20.04".into())),
                        ..Default::default()
                    }
                    .into(),
                ),
                ..Default::default()
            };

            let merged = base.merge(extended);

            assert_eq!(
                merged,
                Image {
                    from: OptionalField::Present(
                        ImageName {
                            path: OptionalField::Present("ubuntu".into()),
                            version: OptionalField::Present(crate::ImageVersion::Tag("20.04".into())),
                            ..Default::default()
                        }
                        .into()
                    ),
                    ..Default::default()
                }
            );
        }
    }
}
