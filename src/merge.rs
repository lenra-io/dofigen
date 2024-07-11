use crate::dofigen_struct::*;
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{
    de::{self, Error as DeError, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_yaml::Value;
use std::{fmt, ops::Deref, str::FromStr};

pub trait Merge {
    fn merge(&self, other: Self) -> Self;
}

// TODO: to merge properly, we need to save the original keys present in the file structure.
// Maybe defining a box that saves the original keys and then deserialize the struct.
// This would permit to remove the Option<> from the required fields.

// Another solution would be to have a different structure for deserializing and the final one.
// This would also remove the permissive structs in the final one.

impl Merge for Value {
    fn merge(&self, other: Self) -> Self {
        match (self, other) {
            (Value::Mapping(a), Value::Mapping(b)) => {
                let mut merged = a.clone();
                for (key, value) in b {
                    merged.insert(key.clone(), a.get(key).unwrap_or(&Value::Null).merge(value));
                }
                Value::Mapping(merged)
            }
            (_, Value::Null) => self.clone(),
            (_, other) => other,
        }
    }
}

macro_rules! impl_Merge {
	(for $($t:ty),+) => {
			$(impl Merge for $t {
				fn merge(&self, other: Self) -> Self {
						let a = serde_yaml::to_value(self).unwrap();
						let b = serde_yaml::to_value(other).unwrap();
						serde_yaml::from_value(a.merge(b)).unwrap()
				}
			})*
	}
}

impl_Merge!(for Builder, Image, Root);

#[cfg(test)]
mod test {
    use super::*;

    mod image {
        use crate::ImageName;

        use super::*;

        #[test]
        fn extends_from_tag() {
            let base = Image {
                from: Some(
                    ImageName {
                        path: Some("ubuntu".into()),
                        ..Default::default()
                    }
                    .into(),
                ),
                ..Default::default()
            };

            let extended = Image {
                from: Some(
                    ImageName {
                        version: Some(crate::ImageVersion::Tag("20.04".into())),
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
                    from: Some(
                        ImageName {
                            path: Some("ubuntu".into()),
                            version: Some(crate::ImageVersion::Tag("20.04".into())),
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
