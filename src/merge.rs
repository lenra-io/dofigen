use serde_yaml::Value;

use crate::dofigen_struct::*;

pub trait Merge {
    fn merge(&self, other: Self) -> Self;
}

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
