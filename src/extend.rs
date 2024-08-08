#[cfg(feature = "permissive")]
use crate::OneOrManyVec;
use crate::{Error, Resource, Result};
use core::fmt;
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{
    de::{self, DeserializeOwned, MapAccess, Visitor},
    Deserialize,
};
use serde_yaml::{from_value, Value};
use std::{collections::HashMap, fs};
use struct_patch::Patch;

const MAX_LOAD_STACK_SIZE: usize = 10;

/// Extends a list of resources with a patch
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema), schemars(default))]
pub struct Extend<T: Default> {
    pub extend: Vec<Resource>,
    pub value: T,
}

impl<'de, T> Deserialize<'de> for Extend<T>
where
    T: Default + DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Extend<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ExtendVisitor<T>(Option<T>);

        impl<'de, T> Visitor<'de> for ExtendVisitor<T>
        where
            T: Default + DeserializeOwned,
        {
            type Value = Extend<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // get the extends from the key 'extend' or 'extends'
                // Parse all the other keys and values as the Extend value
                // TODO: implement without Value ?
                let val: Value =
                    Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
                if let Value::Mapping(mut value_map) = val {
                    let keys = value_map
                        .keys()
                        .filter(|key| {
                            key.as_str()
                                .map(|str| match str.to_lowercase().as_str() {
                                    "extend" | "extends" => true,
                                    _ => false,
                                })
                                .unwrap_or(false)
                        })
                        .collect::<Vec<_>>();
                    if keys.len() > 1 {
                        return Err(serde::de::Error::custom(
                            "Only one of 'extend' or 'extends' is allowed",
                        ));
                    }
                    let extend: Vec<Resource> = if keys.len() > 0 {
                        let extend = value_map
                            .remove(keys[0].clone())
                            .expect("The first element must be found if the array is not empty...");

                        #[cfg(feature = "permissive")]
                        {
                            let one_or_many_vec: OneOrManyVec<Resource> = from_value(extend)
                                .map_err(|error| {
                                    serde::de::Error::custom(format!(
                                        "Deserialize error: {:?}",
                                        error
                                    ))
                                })?;
                            one_or_many_vec.0
                        }

                        #[cfg(not(feature = "permissive"))]
                        from_value(extend).map_err(|error| {
                            serde::de::Error::custom(format!("Deserialize error: {:?}", error))
                        })?
                    } else {
                        vec![]
                    };
                    Ok(Extend {
                        extend,
                        value: from_value(Value::Mapping(value_map))
                            .map_err(|err| serde::de::Error::custom(format!("{}", err)))?,
                    })
                } else {
                    Err(serde::de::Error::custom("Expected a map"))
                }
            }
        }

        let visitor: ExtendVisitor<T> = ExtendVisitor(None);

        deserializer.deserialize_map(visitor)
    }
}

impl<P> Extend<P>
where
    P: Default + DeserializeOwned + Clone,
{
    pub fn merge<T>(&self, context: &mut LoadContext) -> Result<T>
    where
        T: Patch<P> + From<P>,
        P: Default,
    {
        if self.extend.is_empty() {
            return Ok(self.value.clone().into());
        }

        // load extends files
        let mut patches: Vec<T> = self
            .extend
            .iter()
            .map(|extend| {
                let ret = extend.load::<Self>(context)?.merge(context)?;
                context.load_resource_stack.pop();
                Ok(ret)
            })
            .collect::<Result<Vec<_>>>()?;

        // for each extends file, merge it with self
        let mut merged = patches.remove(0);
        for patch in patches {
            merged.apply(patch.into_patch());
        }
        merged.apply(self.value.clone());
        Ok(merged)
    }
}

pub struct LoadContext {
    load_resource_stack: Vec<Resource>,
    resources: HashMap<Resource, String>,
}

impl LoadContext {
    pub fn new() -> Self {
        Self {
            load_resource_stack: vec![],
            resources: HashMap::new(),
        }
    }

    pub fn from_resource(resource: Resource) -> Self {
        Self {
            load_resource_stack: vec![resource],
            resources: HashMap::new(),
        }
    }
}

impl Resource {
    fn load_resource_content(&self, context: &mut LoadContext) -> Result<String> {
        let resource = match self {
            Resource::File(path) => {
                if path.is_absolute() {
                    Resource::File(path.clone())
                } else {
                    if let Some(current_resource) = context.load_resource_stack.last() {
                        match current_resource {
                            Resource::File(file) => Resource::File(
                                file.parent()
                                    .ok_or(Error::Custom(format!(
                                        "The current resource does not have parent dir {:?}",
                                        file
                                    )))?
                                    .join(path),
                            ),
                            Resource::Url(url) => {
                                Resource::Url(url.join(path.to_str().unwrap()).unwrap())
                            }
                        }
                    } else {
                        Resource::File(path.canonicalize().unwrap())
                    }
                }
            }
            Resource::Url(url) => Resource::Url(url.clone()),
        };
        if context.load_resource_stack.contains(&resource) {
            // push the resource to format the error message
            context.load_resource_stack.push(resource.clone());
            return Err(Error::Custom(format!(
                "Circular dependency detected while loading resource {}",
                context
                    .load_resource_stack
                    .iter()
                    .map(|r| format!("{:?}", r))
                    .collect::<Vec<_>>()
                    .join(" -> "),
            )));
        }

        // push the resource to the stack
        context.load_resource_stack.push(resource.clone());

        // check the stack size
        if context.load_resource_stack.len() > MAX_LOAD_STACK_SIZE {
            return Err(Error::Custom(format!(
                "Max load stack size exceeded while loading resource {}",
                context
                    .load_resource_stack
                    .iter()
                    .map(|r| format!("{:?}", r))
                    .collect::<Vec<_>>()
                    .join(" -> "),
            )));
        }

        // load the resource content
        match resource.clone() {
            Resource::File(path) => {
                if let Some(value) = context.resources.get(&resource) {
                    Ok(value.clone())
                } else {
                    let str = fs::read_to_string(path.clone()).map_err(|err| {
                        Error::Custom(format!("Could not read file {:?}: {}", path, err))
                    })?;
                    context.resources.insert(resource, str.clone());
                    Ok(str)
                }
            }
            Resource::Url(url) => {
                if let Some(value) = context.resources.get(&resource) {
                    Ok(value.clone())
                } else {
                    let response = reqwest::blocking::get(url.as_ref()).map_err(|err| {
                        Error::Custom(format!("Could not get url {:?}: {}", url, err))
                    })?;
                    Ok(response.text().map_err(|err| {
                        Error::Custom(format!(
                            "Could not read response from url {:?}: {}",
                            url, err
                        ))
                    })?)
                }
            }
        }
    }

    pub fn load<T>(&self, context: &mut LoadContext) -> Result<T>
    where
        T: DeserializeOwned,
    {
        Ok(
            serde_yaml::from_str(self.load_resource_content(context)?.as_str()).map_err(|err| {
                Error::Custom(format!(
                    "Could not deserialize resource {:?}: {}",
                    self, err
                ))
            })?,
        )
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions_sorted::assert_eq_sorted;

    use super::*;

    mod deserialize {
        use super::*;

        mod extend {

            use super::*;

            #[derive(Deserialize, Patch)]
            #[patch(
                attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
                attribute(serde(deny_unknown_fields, default))
            )]
            struct TestStruct {
                pub name: Option<String>,
                #[serde(flatten)]
                #[patch(name = "TestSubStructPatch", attribute(serde(flatten)))]
                pub sub: TestSubStruct,
            }

            #[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
            #[patch(
                attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
                attribute(serde(deny_unknown_fields, default))
            )]
            struct TestSubStruct {
                pub level: u16,
            }

            #[test]
            fn empty() {
                let data = r#"{}"#;

                let extend_image: Extend<TestStructPatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend_image,
                    Extend {
                        extend: vec![],
                        value: TestStructPatch {
                            sub: Some(TestSubStructPatch::default()),
                            ..Default::default()
                        }
                    }
                );
            }

            #[test]
            fn only_name() {
                let data = "name: ok";

                let extend: Extend<TestStructPatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend,
                    Extend {
                        extend: vec![],
                        value: TestStructPatch {
                            name: Some(Some("ok".into())),
                            sub: Some(TestSubStructPatch::default()),
                            ..Default::default()
                        }
                    }
                );
            }

            #[test]
            fn only_sub() {
                let data = "level: 1";

                let extend: Extend<TestStructPatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend,
                    Extend {
                        extend: vec![],
                        value: TestStructPatch {
                            sub: Some(TestSubStructPatch {
                                level: Some(1),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }
                    }
                );
            }
        }

        mod extend_image {
            use crate::{ImageNamePatch, ImagePatch, StagePatch};

            use super::*;

            #[test]
            fn empty() {
                let data = r#"{}"#;

                let extend_image: Extend<ImagePatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend_image,
                    Extend {
                        extend: vec![],
                        value: ImagePatch {
                            stage: Some(StagePatch::default()),
                            ..Default::default()
                        }
                    }
                );
            }

            #[test]
            fn only_from() {
                let data = r#"
from:
  path: ubuntu
"#;

                let extend_image: Extend<ImagePatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend_image,
                    Extend {
                        extend: vec![],
                        value: ImagePatch {
                            stage: Some(StagePatch {
                                from: Some(Some(
                                    ImageNamePatch {
                                        path: Some("ubuntu".into()),
                                        version: Some(None),
                                        ..Default::default()
                                    }
                                    .into() // To manage permissive
                                )),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }
                    }
                );
            }
        }
    }
}
