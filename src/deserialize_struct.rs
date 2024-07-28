// #[cfg(feature = "permissive")]
// use crate::serde_permissive::{OneOrManyVec as Vec, ParsableStruct};
use serde::{
    de::{self, DeserializeOwned, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_yaml;
use serde_yaml::Value;
use std::fmt;
use std::{collections::BTreeMap, ops::Deref, usize};
use struct_patch::Patch;

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

#[derive(Debug, Clone, PartialEq, Default)]
// #[derive(Deserialize)]
// #[serde(from = "Option<T>")]
pub struct OptionPatch<T>(Option<T>);

impl<T, P> Patch<OptionPatch<P>> for Option<T>
where
    T: Patch<P> + Default + Clone,
{
    fn apply(&mut self, patch: OptionPatch<P>) {
        match self {
            Some(value) => match patch.0 {
                Some(patch_value) => {
                    value.apply(patch_value);
                }
                None => {
                    *self = None;
                }
            },
            None => {
                if let Some(patch_value) = patch.0 {
                    let mut value = T::default();
                    value.apply(patch_value);
                    *self = Some(value);
                }
            }
        }
    }

    fn into_patch(self) -> OptionPatch<P> {
        match self {
            Some(value) => OptionPatch(Some(value.into_patch())),
            None => OptionPatch(None),
        }
    }

    fn into_patch_by_diff(self, previous_struct: Self) -> OptionPatch<P> {
        todo!()
    }

    fn new_empty_patch() -> OptionPatch<P> {
        OptionPatch(None)
    }
}

impl<T> From<Option<T>> for OptionPatch<T> {
    fn from(value: Option<T>) -> Self {
        OptionPatch(value)
    }
}

impl<'de, T> Deserialize<'de> for OptionPatch<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<OptionPatch<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Option<T> = Deserialize::deserialize(deserializer)?;
        Ok(OptionPatch(value))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum VecPatchCommand<T> {
    ReplaceAll(Vec<T>),
    Replace(usize, T),
    InsertBefore(usize, Vec<T>),
    InsertAfter(usize, Vec<T>),
    Append(Vec<T>),
}

/// Patch for Vec<T> that handle some commands based on the position:
/// - `_` to replace the whole list
/// - `+` to append to the list
/// - `n` to replace the nth element
/// - `n+` to append to the nth element
/// - `+n` to prepend to the nth element
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VecPatch<T> {
    commands: Vec<VecPatchCommand<T>>,
}

impl<T> From<Vec<T>> for VecPatch<T> {
    fn from(value: Vec<T>) -> Self {
        Self {
            commands: vec![VecPatchCommand::ReplaceAll(value)],
        }
    }
}

impl<'de, T> Deserialize<'de> for VecPatch<T>
where
    T: Clone + DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<VecPatch<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_vec_patch(deserializer)
    }
}

fn deserialize_vec_patch<'de, D, T>(deserializer: D) -> Result<VecPatch<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Clone + DeserializeOwned,
{
    struct VecPatchVisitor<T>(Option<T>);

    impl<'de, T> Visitor<'de> for VecPatchVisitor<T>
    where
        T: Clone + DeserializeOwned,
    {
        type Value = VecPatch<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence or a map")
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let replacer: Vec<T> =
                Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))?;
            Ok(replacer.into_patch())
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut patch: VecPatch<T> = Vec::new_empty_patch();

            while let Some(key) = map.next_key()? {
                match key {
                    StringOrNumber::String(key) => match key.as_str() {
                        "_" => {
                            patch
                                .commands
                                .push(VecPatchCommand::ReplaceAll(map.next_value()?));
                        }
                        "+" => {
                            patch
                                .commands
                                .push(VecPatchCommand::Append(map.next_value()?));
                        }
                        key => {
                            if key.starts_with('+') {
                                let pos = key[..key.len() - 1].parse::<usize>().unwrap();
                                patch
                                    .commands
                                    .push(VecPatchCommand::InsertBefore(pos, map.next_value()?));
                            } else if key.ends_with('+') {
                                let pos = key[..key.len() - 1].parse::<usize>().unwrap();
                                patch
                                    .commands
                                    .push(VecPatchCommand::InsertAfter(pos, map.next_value()?));
                            } else {
                                let pos = key.parse::<usize>().unwrap();
                                patch
                                    .commands
                                    .push(VecPatchCommand::Replace(pos, map.next_value()?));
                            }
                        }
                    },
                    StringOrNumber::Number(pos) => {
                        patch
                            .commands
                            .push(VecPatchCommand::Replace(pos, map.next_value()?));
                    }
                }
            }
            Ok(patch)
        }
    }

    let visitor: VecPatchVisitor<T> = VecPatchVisitor(None);

    deserializer.deserialize_any(visitor)
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    Number(usize),
}

impl<T> Patch<VecPatch<T>> for Vec<T>
where
    T: Clone,
{
    fn apply(&mut self, patch: VecPatch<T>) {
        let mut commands = patch.commands.clone();
        commands.sort_by(|a, b| match (a, b) {
            (VecPatchCommand::ReplaceAll(_), _) => std::cmp::Ordering::Less,
            (_, VecPatchCommand::ReplaceAll(_)) => std::cmp::Ordering::Greater,
            (VecPatchCommand::InsertBefore(a, _), VecPatchCommand::InsertBefore(b, _))
            | (VecPatchCommand::Replace(a, _), VecPatchCommand::Replace(b, _))
            | (VecPatchCommand::InsertAfter(a, _), VecPatchCommand::InsertAfter(b, _)) => a.cmp(b),
            (VecPatchCommand::Replace(a, _), VecPatchCommand::InsertAfter(b, _)) => {
                match a.cmp(b) {
                    std::cmp::Ordering::Equal => std::cmp::Ordering::Less,
                    other => other,
                }
            }
            (VecPatchCommand::InsertAfter(a, _), VecPatchCommand::Replace(b, _)) => {
                match a.cmp(b) {
                    std::cmp::Ordering::Equal => std::cmp::Ordering::Greater,
                    other => other,
                }
            }
            (VecPatchCommand::InsertBefore(_, _), _) => std::cmp::Ordering::Less,
            (_, VecPatchCommand::InsertBefore(_, _)) => std::cmp::Ordering::Greater,
            (VecPatchCommand::Append(_), _) => std::cmp::Ordering::Greater,
            (_, VecPatchCommand::Append(_)) => std::cmp::Ordering::Less,
        });
        let mut reseted = false;
        let mut last_modified_position: usize = usize::MAX;
        // initial array length
        let initial_len = self.len();
        // save the number of elements added before the positions
        let mut adapted_positions: Vec<usize> = vec![0; self.len()];
        // save the current position and corresponding adapted position to avoid recomputing it
        let mut current_position: (usize, usize) = (0, 0);

        for command in commands {
            match command {
                VecPatchCommand::ReplaceAll(elements) => {
                    if reseted {
                        panic!("Cannot replace the list twice");
                    }
                    reseted = true;
                    self.clear();
                    self.extend(elements.into_iter());
                }
                VecPatchCommand::Replace(pos, elements) => {
                    if reseted {
                        panic!("Cannot replace element at position {} after a reset", pos);
                    }
                    if pos >= initial_len {
                        panic!("Position {} is out of bounds", pos);
                    }
                    if pos == last_modified_position {
                        panic!("Cannot replace element at position {} after another modification on it", pos);
                    }
                    for i in current_position.0..pos {
                        current_position.0 = i;
                        current_position.1 += adapted_positions[i + 1];
                    }
                    self[current_position.1] = elements;
                    last_modified_position = pos;
                }
                VecPatchCommand::InsertBefore(pos, elements) => {
                    if reseted {
                        panic!(
                            "Cannot insert before element at position {} after a reset",
                            pos
                        );
                    }
                    if pos >= initial_len {
                        panic!("Position {} is out of bounds", pos);
                    }
                    for i in current_position.0..pos {
                        current_position.0 = i;
                        current_position.1 += adapted_positions[i + 1];
                    }
                    let added = elements.len();
                    self.splice(current_position.1..current_position.1, elements);
                    current_position.1 += added;
                    adapted_positions[pos as usize] += added;
                }
                VecPatchCommand::InsertAfter(pos, elements) => {
                    if reseted {
                        panic!(
                            "Cannot insert after element at position {} after a reset",
                            pos
                        );
                    }
                    if pos >= initial_len {
                        panic!("Position {} is out of bounds", pos);
                    }
                    for i in current_position.0..pos {
                        current_position.0 = i;
                        current_position.1 += adapted_positions[i + 1];
                    }
                    let usize_pos = current_position.1 + 1;
                    let added = elements.len();
                    self.splice(usize_pos..usize_pos, elements);
                    if pos + 1 < initial_len {
                        adapted_positions[(pos + 1) as usize] = added;
                    } else {
                        adapted_positions.push(added);
                    }
                }
                VecPatchCommand::Append(elements) => {
                    self.extend(elements);
                }
            }
        }
    }

    fn into_patch(self) -> VecPatch<T> {
        VecPatch {
            commands: vec![VecPatchCommand::ReplaceAll(self)],
        }
    }

    fn into_patch_by_diff(self, previous_struct: Self) -> VecPatch<T> {
        todo!()
    }

    fn new_empty_patch() -> VecPatch<T> {
        VecPatch { commands: vec![] }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum VecDeepPatchCommand<T, P>
where
    T: Clone + Patch<P>,
{
    ReplaceAll(Vec<T>),
    Replace(usize, T),
    Patch(usize, P),
    InsertBefore(usize, Vec<T>),
    InsertAfter(usize, Vec<T>),
    Append(Vec<T>),
}

/// Patch for Vec<T> that handle some commands based on the position:
/// - `_` to replace the whole list
/// - `+` to append to the list
/// - `n` to replace the nth element
/// - `n<` to patch the nth element
/// - `n+` to append to the nth element
/// - `+n` to prepend to the nth element
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VecDeepPatch<T, P>
where
    T: Clone + Patch<P>,
{
    commands: Vec<VecDeepPatchCommand<T, P>>,
}

impl<T, P> From<Vec<T>> for VecDeepPatch<T, P>
where
    T: Clone + Patch<P>,
{
    fn from(value: Vec<T>) -> Self {
        Self {
            commands: vec![VecDeepPatchCommand::ReplaceAll(value)],
        }
    }
}

impl<T, P> Patch<VecDeepPatch<T, P>> for Vec<T>
where
    T: Clone + Patch<P>,
    P: Clone,
{
    fn apply(&mut self, patch: VecDeepPatch<T, P>) {
        let mut commands = patch.commands.clone();
        commands.sort_by(|a, b| match (a, b) {
            (VecDeepPatchCommand::ReplaceAll(_), _) => std::cmp::Ordering::Less,
            (_, VecDeepPatchCommand::ReplaceAll(_)) => std::cmp::Ordering::Greater,
            (VecDeepPatchCommand::InsertBefore(a, _), VecDeepPatchCommand::InsertBefore(b, _))
            | (
                VecDeepPatchCommand::Replace(a, _) | VecDeepPatchCommand::Patch(a, _),
                VecDeepPatchCommand::Replace(b, _) | VecDeepPatchCommand::Patch(b, _),
            )
            | (VecDeepPatchCommand::InsertAfter(a, _), VecDeepPatchCommand::InsertAfter(b, _)) => {
                a.cmp(b)
            }
            (
                VecDeepPatchCommand::Replace(a, _) | VecDeepPatchCommand::Patch(a, _),
                VecDeepPatchCommand::InsertAfter(b, _),
            ) => match a.cmp(b) {
                std::cmp::Ordering::Equal => std::cmp::Ordering::Less,
                other => other,
            },
            (
                VecDeepPatchCommand::InsertAfter(a, _),
                VecDeepPatchCommand::Replace(b, _) | VecDeepPatchCommand::Patch(b, _),
            ) => match a.cmp(b) {
                std::cmp::Ordering::Equal => std::cmp::Ordering::Greater,
                other => other,
            },
            (VecDeepPatchCommand::InsertBefore(_, _), _) => std::cmp::Ordering::Less,
            (_, VecDeepPatchCommand::InsertBefore(_, _)) => std::cmp::Ordering::Greater,
            (VecDeepPatchCommand::Append(_), _) => std::cmp::Ordering::Greater,
            (_, VecDeepPatchCommand::Append(_)) => std::cmp::Ordering::Less,
        });
        let mut reseted = false;
        let mut last_modified_position: usize = usize::MAX;
        // initial array length
        let initial_len = self.len();
        // save the number of elements added before the positions
        let mut adapted_positions: Vec<usize> = vec![0; self.len()];
        // save the current position and corresponding adapted position to avoid recomputing it
        let mut current_position: (usize, usize) = (0, 0);

        for command in commands {
            match command {
                VecDeepPatchCommand::ReplaceAll(elements) => {
                    if reseted {
                        panic!("Cannot replace the list twice");
                    }
                    reseted = true;
                    self.clear();
                    self.extend(elements.into_iter());
                }
                VecDeepPatchCommand::Replace(pos, elements) => {
                    if reseted {
                        panic!("Cannot replace element at position {} after a reset", pos);
                    }
                    if pos >= initial_len {
                        panic!("Position {} is out of bounds", pos);
                    }
                    if pos == last_modified_position {
                        panic!("Cannot replace element at position {} after another modification on it", pos);
                    }
                    for i in current_position.0..pos {
                        current_position.0 = i;
                        current_position.1 += adapted_positions[i + 1];
                    }
                    self[current_position.1] = elements;
                    last_modified_position = pos;
                }
                VecDeepPatchCommand::Patch(pos, element) => {
                    if reseted {
                        panic!("Cannot patch element at position {} after a reset", pos);
                    }
                    if pos >= initial_len {
                        panic!("Position {} is out of bounds", pos);
                    }
                    if pos == last_modified_position {
                        panic!(
                            "Cannot patch element at position {} after another modification on it",
                            pos
                        );
                    }
                    for i in current_position.0..pos {
                        current_position.0 = i;
                        current_position.1 += adapted_positions[i + 1];
                    }
                    self[current_position.1].apply(element);
                    last_modified_position = pos;
                }
                VecDeepPatchCommand::InsertBefore(pos, elements) => {
                    if reseted {
                        panic!(
                            "Cannot insert before element at position {} after a reset",
                            pos
                        );
                    }
                    if pos >= initial_len {
                        panic!("Position {} is out of bounds", pos);
                    }
                    for i in current_position.0..pos {
                        current_position.0 = i;
                        current_position.1 += adapted_positions[i + 1];
                    }
                    let added = elements.len();
                    self.splice(current_position.1..current_position.1, elements);
                    current_position.1 += added;
                    adapted_positions[pos as usize] += added;
                }
                VecDeepPatchCommand::InsertAfter(pos, elements) => {
                    if reseted {
                        panic!(
                            "Cannot insert after element at position {} after a reset",
                            pos
                        );
                    }
                    if pos >= initial_len {
                        panic!("Position {} is out of bounds", pos);
                    }
                    for i in current_position.0..pos {
                        current_position.0 = i;
                        current_position.1 += adapted_positions[i + 1];
                    }
                    let usize_pos = current_position.1 + 1;
                    let added = elements.len();
                    self.splice(usize_pos..usize_pos, elements);
                    if pos + 1 < initial_len {
                        adapted_positions[(pos + 1) as usize] = added;
                    } else {
                        adapted_positions.push(added);
                    }
                }
                VecDeepPatchCommand::Append(elements) => {
                    self.extend(elements);
                }
            }
        }
    }

    fn into_patch(self) -> VecDeepPatch<T, P> {
        VecDeepPatch {
            commands: vec![VecDeepPatchCommand::ReplaceAll(self)],
        }
    }

    fn into_patch_by_diff(self, previous_struct: Self) -> VecDeepPatch<T, P> {
        todo!()
    }

    fn new_empty_patch() -> VecDeepPatch<T, P> {
        VecDeepPatch { commands: vec![] }
    }
}

impl<'de, T, P> Deserialize<'de> for VecDeepPatch<T, P>
where
    T: Clone + DeserializeOwned + Default + Patch<P>,
    P: Clone + DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_vec_deep_patch(deserializer)
    }
}

fn deserialize_vec_deep_patch<'de, D, T, P>(deserializer: D) -> Result<VecDeepPatch<T, P>, D::Error>
where
    D: Deserializer<'de>,
    T: Clone + DeserializeOwned + Default + Patch<P>,
    P: Clone + DeserializeOwned,
{
    struct VecDeepPatchVisitor<T, P>(Option<BTreeMap<T, P>>);

    impl<'de, T, P> Visitor<'de> for VecDeepPatchVisitor<T, P>
    where
        T: Clone + DeserializeOwned + Patch<P> + Default,
        P: Clone + DeserializeOwned,
    {
        type Value = VecDeepPatch<T, P>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence or a map")
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let replacer: Vec<T> =
                Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))?;
            Ok(replacer.into_patch())
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut patch: VecDeepPatch<T, P> = Vec::new_empty_patch();

            while let Some(key) = map.next_key()? {
                match key {
                    StringOrNumber::String(key) => match key.as_str() {
                        "_" => {
                            patch
                                .commands
                                .push(VecDeepPatchCommand::ReplaceAll(map.next_value()?));
                        }
                        "+" => {
                            patch
                                .commands
                                .push(VecDeepPatchCommand::Append(map.next_value()?));
                        }
                        key => {
                            if key.starts_with('+') {
                                let pos = key[..key.len() - 1].parse::<usize>().unwrap();
                                patch.commands.push(VecDeepPatchCommand::InsertBefore(
                                    pos,
                                    map.next_value()?,
                                ));
                            } else if key.ends_with('+') {
                                let pos = key[..key.len() - 1].parse::<usize>().unwrap();
                                patch
                                    .commands
                                    .push(VecDeepPatchCommand::InsertAfter(pos, map.next_value()?));
                            } else if key.ends_with('<') {
                                let pos = key[..key.len() - 1].parse::<usize>().unwrap();
                                patch
                                    .commands
                                    .push(VecDeepPatchCommand::Patch(pos, map.next_value()?));
                            } else {
                                let pos = key.parse::<usize>().unwrap();
                                patch
                                    .commands
                                    .push(VecDeepPatchCommand::Replace(pos, map.next_value()?));
                            }
                        }
                    },
                    StringOrNumber::Number(pos) => {
                        patch
                            .commands
                            .push(VecDeepPatchCommand::Replace(pos, map.next_value()?));
                    }
                }
            }
            Ok(patch)
        }
    }

    let visitor: VecDeepPatchVisitor<T, P> = VecDeepPatchVisitor(None);

    deserializer.deserialize_any(visitor)
}

#[cfg(test)]
mod test {
    use super::*;

    mod base_patch {

        use super::*;
        use pretty_assertions_sorted::assert_eq_sorted;
        use serde::Deserialize;
        use struct_patch::Patch;

        #[derive(Deserialize, Debug, Clone, PartialEq, Patch, Default)]
        #[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
        struct TestStruct {
            pub name: String,
            #[patch_name = "OptionPatch<SubTestStructPatch>"]
            pub sub: Option<SubTestStruct>,
        }

        #[derive(Deserialize, Debug, Clone, PartialEq, Patch, Default)]
        #[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
        struct SubTestStruct {
            #[patch_name = "VecPatch<String>"]
            pub list: Vec<String>,
            pub num: Option<u32>,
        }

        #[derive(Deserialize, Patch, Default)]
        #[patch_derive(Deserialize, Default)]
        struct MyTestStruct<T>
        where
            T: PartialEq,
        {
            pub num: Option<T>,
        }

        #[test]
        fn test_simple_patch() {
            let base = r#"
                name: patch1
                sub:
                  list:
                    - item1
                    - item2
                  num: 42
            "#;

            let patch = r#"
                name: patch2
                sub:
                    num: 43
            "#;

            let mut base_data: TestStruct = serde_yaml::from_str(base).unwrap();
            let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

            base_data.apply(patch_data);

            assert_eq_sorted!(
                base_data,
                TestStruct {
                    name: "patch2".into(),
                    sub: Some(SubTestStruct {
                        list: vec!["item1".into(), "item2".into()],
                        num: Some(43)
                    })
                }
            );
        }

        #[test]
        fn test_vec_replace() {
            let base = r#"
                name: patch1
                sub:
                  list:
                    - item1
                    - item2
                  num: 42
            "#;

            let patch = r#"
                sub:
                  list:
                    - item3
                    - item4
            "#;

            let mut base_data: TestStruct = serde_yaml::from_str(base).unwrap();
            let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

            base_data.apply(patch_data);

            assert_eq_sorted!(
                base_data,
                TestStruct {
                    name: "patch1".into(),
                    sub: Some(SubTestStruct {
                        list: vec!["item3".into(), "item4".into()],
                        num: Some(42)
                    })
                }
            );
        }

        #[test]
        fn test_vec_append_patch() {
            let base = r#"
                name: patch1
                sub:
                  list:
                    - item1
                    - item2
                  num: 42
            "#;

            let patch = r#"
                sub:
                    list:
                        +:
                            - item3
                            - item4
            "#;

            let mut base_data: TestStruct = serde_yaml::from_str(base).unwrap();
            let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

            base_data.apply(patch_data);

            assert_eq_sorted!(
                base_data,
                TestStruct {
                    name: "patch1".into(),
                    sub: Some(SubTestStruct {
                        list: vec![
                            "item1".into(),
                            "item2".into(),
                            "item3".into(),
                            "item4".into()
                        ],
                        num: Some(42)
                    })
                }
            );
        }

        #[test]
        fn test_vec_replace_patch() {
            let base = r#"
                name: patch1
                sub:
                  list:
                    - item1
                    - item2
                  num: 42
            "#;

            let patch = r#"
                sub:
                    list:
                        0: item3
            "#;

            let mut base_data: TestStruct = serde_yaml::from_str(base).unwrap();
            let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

            base_data.apply(patch_data);

            assert_eq_sorted!(
                base_data,
                TestStruct {
                    name: "patch1".into(),
                    sub: Some(SubTestStruct {
                        list: vec!["item3".into(), "item2".into()],
                        num: Some(42)
                    })
                }
            );
        }
    }
}
