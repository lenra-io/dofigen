use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_yaml::Value;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeserializableStruct<T>
where
    T: Sized,
{
    fields: Vec<String>,
    content: T,
}

impl<T: Sized> Deref for DeserializableStruct<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.content
    }
}

impl<'de, T> Deserialize<'de> for DeserializableStruct<T>
where
    T: Sized + DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<DeserializableStruct<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        let fields = if value.is_mapping() {
            value
                .as_mapping()
                .unwrap()
                .keys()
                .map(|key| key.as_str().unwrap().to_string())
                .collect()
        } else {
            vec![]
        };
        // TODO: fix the error management
        let content = Deserialize::deserialize(value).unwrap();
        Ok(DeserializableStruct { fields, content })
    }
}

// The use of proc-macro2 is necessary to have more permissive macros. Here is a sample project: https://github.com/mcilloni/proc-macro-sample/tree/master/load-dump-derive

// macro_rules! Deserializable
// {
//     (
//         pub struct $name:ident {
//             $(
//                 $( #[$attrs:meta] )*
//                 $field_name:ident : $field_type:ty,
//             )*
//         }
//     ) =>
//     {
// 				pub struct $name
// 				{
// 						$(
// 								$( #[$attrs] )*
// 								pub $field_name : $field_type,
// 						)*
// 				}

// 				#[derive(Debug, Clone, serde::)]
// 				#[serde(rename_all = "camelCase")]
// 				pub struct concat_idents!($name, Deserializable)
// 				{
// 						$(
// 								$( #[$attrs] )*
// 								pub $field_name : $field_type,
// 						)*
// 				}
//     };
// }

// Deserializable!
// {
// 		pub struct Test {
// 				field1: String,
// 				field2: u32,
// 		}
// }
