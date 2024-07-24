use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_yaml::Value;
use std::ops::Deref;

use serde_yaml;

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

// #[cfg(test)]
// mod test {
//     use super::*;

//     mod base_patch {
//         use std::collections::HashMap;

//         use super::*;
//         use serde::Deserialize;
//         use struct_patch::Patch;

//         #[derive(Deserialize, Debug, Clone, PartialEq, Patch)]
//         #[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
//         struct TestStruct {
//             pub name: String,
//             #[patch_name = "Option<SubTestStructPatch>"]
//             pub sub: Option<SubTestStruct>,
//         }

//         #[derive(Deserialize, Debug, Clone, PartialEq, Patch)]
//         #[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
//         struct SubTestStruct {
//             // #[patch_name = "VecPatch<String>"]
//             pub list: Vec<String>,
//             pub num: Option<u32>,
//         }

//         // #[derive(Deserialize, Patch)]
//         // #[patch_derive(Deserialize, Default)]
//         // struct MyTestStruct<T> {
//         //     pub num: Option<T>,
//         // }

//         #[derive(Debug, Clone, PartialEq, Default)]
//         struct VecPatch {
//             pub actions: HashMap<String, Value>,
//         }

//         impl<T> Patch<VecPatch> for Vec<T> {
//             fn apply(&mut self, patch: VecPatch) {
//                 todo!()
//             }

//             fn into_patch(self) -> VecPatch {
//                 todo!()
//             }

//             fn into_patch_by_diff(self, previous_struct: Self) -> VecPatch {
//                 todo!()
//             }

//             fn new_empty_patch() -> VecPatch {
//                 todo!()
//             }
//         }

//         #[test]
//         fn test_simple_patch() {
//             let base = r#"
//                 name: patch1
//                 sub:
//                   list:
//                     - item1
//                     - item2
//                   num: 42
//             "#;

//             let patch = r#"
//                 name: patch2
//                 sub:
//                     num: 43
//             "#;

//             let mut base_data: TestStruct = serde_yaml::from_str(base).unwrap();
//             let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

//             base_data.apply(patch_data);

//             assert_eq_sorted!(base_data.name, "patch2");
//             assert_eq_sorted!(base_data.sub.num, Some(43));
//         }

//         #[test]
//         fn test_vec_patch() {
//             let base = r#"
//                 name: patch1
//                 sub:
//                   list:
//                     - item1
//                     - item2
//                   num: 42
//             "#;

//             let patch = r#"
//                 sub:
//                     list:
//                         >:
//                             - item3
//                             - item4
//             "#;

//             let mut base_data: TestStruct = serde_yaml::from_str(base).unwrap();
//             let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

//             base_data.apply(patch_data);

//             assert_eq_sorted!(base_data.name, "patch1");
//             assert_eq_sorted!(base_data.sub.num, Some(42));
//             assert_eq_sorted!(base_data.sub.list, vec!["item1", "item2", "item3", "item4"]);
//         }

//         #[test]
//         fn test_vec_replace_patch() {
//             let base = r#"
//                 name: patch1
//                 sub:
//                   list:
//                     - item1
//                     - item2
//                   num: 42
//             "#;

//             let patch = r#"
//                 sub:
//                     list:
//                         0: item3
//             "#;

//             let mut base_data: TestStruct = serde_yaml::from_str(base).unwrap();
//             let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

//             base_data.apply(patch_data);

//             assert_eq_sorted!(base_data.name, "patch1");
//             assert_eq_sorted!(base_data.sub.num, Some(42));
//             assert_eq_sorted!(base_data.sub.list, vec!["item3", "item2"]);
//         }
//     }
// }
