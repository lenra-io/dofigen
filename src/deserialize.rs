use crate::dofigen_struct::*;
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{de, Deserialize, Deserializer};
use std::{collections::BTreeMap, fmt, marker::PhantomData, usize};
#[cfg(feature = "permissive")]
use std::{ops::Deref, str::FromStr};
use struct_patch::Patch;

/// Implements the From trait for a struct from a patch
macro_rules! impl_from_patch {
    ($struct:ty, $patch:ty) => {
        impl From<$patch> for $struct {
            fn from(value: $patch) -> Self {
                let mut s = Self::default();
                s.apply(value);
                s
            }
        }
    };
}

impl_from_patch!(Image, ImagePatch);
impl_from_patch!(Stage, StagePatch);
impl_from_patch!(Healthcheck, HealthcheckPatch);
impl_from_patch!(ImageName, ImageNamePatch);
impl_from_patch!(Artifact, ArtifactPatch);
impl_from_patch!(Run, RunPatch);
impl_from_patch!(Port, PortPatch);
impl_from_patch!(User, UserPatch);
impl_from_patch!(Copy, CopyPatch);
impl_from_patch!(Add, AddPatch);
impl_from_patch!(AddGitRepo, AddGitRepoPatch);

//////////////////////// Patch structures ////////////////////////

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum CopyResourcePatch {
    Copy(CopyPatch),
    AddGitRepo(AddGitRepoPatch),
    Add(AddPatch),
    Unknown(UnknownPatch),
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct UnknownPatch {
    #[serde(flatten)]
    pub options: Option<CopyOptionsPatch>,
    pub exclude: Option<VecPatch<String>>,
}

/// A struct that can be parsed from a string
#[cfg(feature = "permissive")]
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct ParsableStruct<T>(pub(crate) T)
where
    T: FromStr;

/// One or many values
#[cfg(feature = "permissive")]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(from = "OneOrManyDeserializable<T>")]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct OneOrMany<T>(pub Vec<T>);

/// Patch for Vec<T> that handle some commands based on the position:
/// - `_` to replace the whole list
/// - `+` to append to the list
/// - `n` to replace the nth element
/// - `n+` to append to the nth element
/// - `+n` to prepend to the nth element
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(from = "VecPatchDeserializable<T>")]
pub struct VecPatch<T>
where
    T: Clone,
{
    commands: Vec<VecPatchCommand<T>>,
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
/// - `n<` to patch the nth element
/// - `n+` to append to the nth element
/// - `+n` to prepend to the nth element
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(
    from = "VecDeepPatchDeserializable<T, P>",
    bound(deserialize = "T: Clone + From<P>, P: Clone + Deserialize<'de>")
)]
pub struct VecDeepPatch<T, P>
where
    T: Clone + Patch<P> + From<P>,
    P: Clone,
{
    commands: Vec<VecDeepPatchCommand<T, P>>,
}

#[derive(Debug, Clone, PartialEq)]
enum VecDeepPatchCommand<T, P>
where
    T: Clone,
{
    ReplaceAll(Vec<T>),
    Replace(usize, T),
    Patch(usize, P),
    InsertBefore(usize, Vec<T>),
    InsertAfter(usize, Vec<T>),
    Append(Vec<T>),
}

//////////////////////// Deserialization structures ////////////////////////

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    Number(usize),
}

/// One or many for deserialization
#[cfg(feature = "permissive")]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
enum OneOrManyDeserializable<T> {
    One(T),
    Many(Vec<T>),
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum VecPatchDeserializable<T>
where
    T: Clone,
{
    #[cfg(feature = "permissive")]
    Vec(OneOrMany<T>),
    #[cfg(not(feature = "permissive"))]
    Vec(Vec<T>),
    Map(VecPatchCommandMap<T>),
}

#[derive(Debug, Clone, PartialEq)]
struct VecPatchCommandMap<T>
where
    T: Clone,
{
    commands: Vec<VecPatchCommand<T>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(
    untagged,
    bound(deserialize = "T: Clone + From<P> + Patch<P>, P: Clone + Deserialize<'de>")
)]
enum VecDeepPatchDeserializable<T, P>
where
    T: Clone + From<P>,
    P: Clone,
{
    Map(VecDeepPatchCommandMap<T, P>),
    #[cfg(feature = "permissive")]
    Vec(OneOrMany<P>),
    #[cfg(not(feature = "permissive"))]
    Vec(Vec<P>),
}

#[derive(Debug, Clone, PartialEq)]
struct VecDeepPatchCommandMap<T, P>
where
    T: Clone,
{
    commands: Vec<VecDeepPatchCommand<T, P>>,
}

//////////////////////// Implementations ////////////////////////

impl Default for CopyResourcePatch {
    fn default() -> Self {
        CopyResourcePatch::Unknown(UnknownPatch::default())
    }
}

impl From<CopyResourcePatch> for CopyResource {
    fn from(patch: CopyResourcePatch) -> Self {
        match patch {
            CopyResourcePatch::Copy(p) => CopyResource::Copy(p.into()),
            CopyResourcePatch::Add(p) => CopyResource::Add(p.into()),
            CopyResourcePatch::AddGitRepo(p) => CopyResource::AddGitRepo(p.into()),
            CopyResourcePatch::Unknown(p) => panic!("Unknown patch: {:?}", p),
        }
    }
}

#[cfg(feature = "permissive")]
impl<T: FromStr> From<T> for ParsableStruct<T> {
    fn from(value: T) -> Self {
        ParsableStruct(value)
    }
}

#[cfg(feature = "permissive")]
impl<T> Default for OneOrMany<T>
where
    T: Sized,
{
    fn default() -> Self {
        OneOrMany(vec![])
    }
}

#[cfg(feature = "permissive")]
impl<T> Deref for OneOrMany<T>
where
    T: Sized,
{
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "permissive")]
impl<T> From<OneOrManyDeserializable<T>> for OneOrMany<T> {
    fn from(value: OneOrManyDeserializable<T>) -> Self {
        match value {
            OneOrManyDeserializable::One(v) => OneOrMany(vec![v]),
            OneOrManyDeserializable::Many(v) => OneOrMany(v),
        }
    }
}

impl<T> From<VecPatchDeserializable<T>> for VecPatch<T>
where
    T: Clone,
{
    fn from(value: VecPatchDeserializable<T>) -> Self {
        match value {
            #[cfg(feature = "permissive")]
            VecPatchDeserializable::Vec(v) => VecPatch {
                commands: vec![VecPatchCommand::ReplaceAll(v.0)],
            },
            #[cfg(not(feature = "permissive"))]
            VecPatchDeserializable::Vec(v) => VecPatch {
                commands: vec![VecPatchCommand::ReplaceAll(v)],
            },
            VecPatchDeserializable::Map(v) => VecPatch {
                commands: v.commands,
            },
        }
    }
}

impl<T, P> From<VecDeepPatchCommand<T, P>> for VecPatchCommand<T>
where
    T: Clone + From<P>,
{
    fn from(value: VecDeepPatchCommand<T, P>) -> Self {
        match value {
            VecDeepPatchCommand::ReplaceAll(v) => VecPatchCommand::ReplaceAll(v),
            VecDeepPatchCommand::Replace(pos, v) => VecPatchCommand::Replace(pos, v),
            VecDeepPatchCommand::Patch(pos, v) => VecPatchCommand::Replace(pos, v.into()),
            VecDeepPatchCommand::InsertBefore(pos, v) => VecPatchCommand::InsertBefore(pos, v),
            VecDeepPatchCommand::InsertAfter(pos, v) => VecPatchCommand::InsertAfter(pos, v),
            VecDeepPatchCommand::Append(v) => VecPatchCommand::Append(v),
        }
    }
}

impl<T, P> From<VecDeepPatchDeserializable<T, P>> for VecDeepPatch<T, P>
where
    T: Clone + Patch<P> + From<P>,
    P: Clone,
{
    fn from(value: VecDeepPatchDeserializable<T, P>) -> Self {
        match value {
            #[cfg(feature = "permissive")]
            VecDeepPatchDeserializable::Vec(v) => VecDeepPatch {
                commands: vec![VecDeepPatchCommand::ReplaceAll(
                    v.0.iter().map(|p| p.clone().into()).collect(),
                )],
            },
            #[cfg(not(feature = "permissive"))]
            VecDeepPatchDeserializable::Vec(v) => VecDeepPatch {
                commands: vec![VecDeepPatchCommand::ReplaceAll(
                    v.iter().map(|p| p.clone().into()).collect(),
                )],
            },
            VecDeepPatchDeserializable::Map(v) => VecDeepPatch {
                commands: v.commands,
            },
        }
    }
}

//////////////////////// Patch ////////////////////////

impl Patch<CopyResourcePatch> for CopyResource {
    fn apply(&mut self, patch: CopyResourcePatch) {
        match (self, patch) {
            (CopyResource::Copy(s), CopyResourcePatch::Copy(p)) => s.apply(p),
            (CopyResource::Add(s), CopyResourcePatch::Add(p)) => s.apply(p),
            (CopyResource::AddGitRepo(s), CopyResourcePatch::AddGitRepo(p)) => s.apply(p),
            _ => todo!(),
        }
    }

    fn into_patch(self) -> CopyResourcePatch {
        match self {
            CopyResource::Copy(s) => CopyResourcePatch::Copy(s.into_patch()),
            CopyResource::Add(s) => CopyResourcePatch::Add(s.into_patch()),
            CopyResource::AddGitRepo(s) => CopyResourcePatch::AddGitRepo(s.into_patch()),
        }
    }

    fn into_patch_by_diff(self, previous_struct: Self) -> CopyResourcePatch {
        match (self, previous_struct) {
            (CopyResource::Copy(s), CopyResource::Copy(p)) => {
                CopyResourcePatch::Copy(s.into_patch_by_diff(p))
            }
            (CopyResource::Add(s), CopyResource::Add(p)) => {
                CopyResourcePatch::Add(s.into_patch_by_diff(p))
            }
            (CopyResource::AddGitRepo(s), CopyResource::AddGitRepo(p)) => {
                CopyResourcePatch::AddGitRepo(s.into_patch_by_diff(p))
            }
            _ => todo!(),
        }
    }

    fn new_empty_patch() -> CopyResourcePatch {
        CopyResourcePatch::default()
    }
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
        let mut reset = false;
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
                    if reset {
                        panic!("Cannot replace the list twice");
                    }
                    reset = true;
                    self.clear();
                    self.extend(elements.into_iter());
                }
                VecPatchCommand::Replace(pos, elements) => {
                    if reset {
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
                    if reset {
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
                    if reset {
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

    fn into_patch_by_diff(self, _previous_struct: Self) -> VecPatch<T> {
        todo!()
        // the diff is computed by comparing the two arrays
        // let mut commands = vec![];

        // VecPatch { commands }
    }

    fn new_empty_patch() -> VecPatch<T> {
        VecPatch { commands: vec![] }
    }
}

impl<T, P> Patch<VecDeepPatch<T, P>> for Vec<T>
where
    T: Clone + Patch<P> + From<P>,
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
        let mut reset = false;
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
                    if reset {
                        panic!("Cannot replace the list twice");
                    }
                    reset = true;
                    self.clear();
                    self.extend(elements.into_iter());
                }
                VecDeepPatchCommand::Replace(pos, element) => {
                    if reset {
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
                    self[current_position.1] = element;
                    last_modified_position = pos;
                }
                VecDeepPatchCommand::Patch(pos, element) => {
                    if reset {
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
                    if reset {
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
                    if reset {
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

    fn into_patch_by_diff(self, _previous_struct: Self) -> VecDeepPatch<T, P> {
        todo!()
    }

    fn new_empty_patch() -> VecDeepPatch<T, P> {
        VecDeepPatch { commands: vec![] }
    }
}

//////////////////////// Deserialize ////////////////////////

#[cfg(feature = "permissive")]
impl<'de, T> Deserialize<'de> for ParsableStruct<T>
where
    T: FromStr + Sized + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<ParsableStruct<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PermissiveStructVisitor<T>(Option<T>);

        impl<'de, T> de::Visitor<'de> for PermissiveStructVisitor<T>
        where
            T: Deserialize<'de> + FromStr,
        {
            type Value = T;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a number, a string or a map")
            }

            fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(v.to_string().as_str())
            }

            fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(v.to_string().as_str())
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(v.to_string().as_str())
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(v.to_string().as_str())
            }

            fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(v.to_string().as_str())
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(v.to_string().as_str())
            }

            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(v.to_string().as_str())
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(v.to_string().as_str())
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(v.to_string().as_str())
            }

            fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(v.to_string().as_str())
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // TODO: improve error management
                v.parse()
                    .map_err(|_| E::custom("Error while parsing a permissive struct"))
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
            }
        }

        let visitor: PermissiveStructVisitor<T> = PermissiveStructVisitor(None);

        deserializer.deserialize_any(visitor).map(ParsableStruct)
    }
}

impl<'de, T> Deserialize<'de> for VecPatchCommandMap<T>
where
    T: Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let commands: Vec<VecDeepPatchCommand<T, T>> =
            deserialize_vec_patch_commands(deserializer)?;
        Ok(VecPatchCommandMap {
            commands: commands.iter().map(|c| c.clone().into()).collect(),
        })
    }
}

impl<'de, T, P> Deserialize<'de> for VecDeepPatchCommandMap<T, P>
where
    T: Clone + From<P>,
    P: Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(VecDeepPatchCommandMap {
            commands: deserialize_vec_patch_commands(deserializer)?,
        })
    }
}

fn deserialize_vec_patch_commands<'de, D, T, P>(
    deserializer: D,
) -> Result<Vec<VecDeepPatchCommand<T, P>>, D::Error>
where
    D: Deserializer<'de>,
    T: Clone + From<P>,
    P: Clone + Deserialize<'de>,
{
    struct VecDeepPatchCommandsVisitor<T, P>(PhantomData<fn() -> BTreeMap<T, P>>);

    fn map_vec<T, P>(patch: Vec<P>) -> Vec<T>
    where
        T: Clone + From<P>,
        P: Clone,
    {
        patch.iter().map(|p| p.clone().into()).collect()
    }

    impl<'de, T, P> de::Visitor<'de> for VecDeepPatchCommandsVisitor<T, P>
    where
        T: Clone + From<P>,
        P: Clone + Deserialize<'de>,
    {
        type Value = Vec<VecDeepPatchCommand<T, P>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            let mut commands = vec![];

            while let Some(key) = map.next_key()? {
                match key {
                    StringOrNumber::String(key) => match key.as_str() {
                        "_" => {
                            commands
                                .push(VecDeepPatchCommand::ReplaceAll(map_vec(map.next_value()?)));
                        }
                        "+" => {
                            commands.push(VecDeepPatchCommand::Append(map_vec(map.next_value()?)));
                        }
                        key => {
                            if key.starts_with('+') {
                                let pos = key[..key.len() - 1].parse::<usize>().unwrap();
                                commands.push(VecDeepPatchCommand::InsertBefore(
                                    pos,
                                    map_vec(map.next_value()?),
                                ));
                            } else if key.ends_with('+') {
                                let pos = key[..key.len() - 1].parse::<usize>().unwrap();
                                commands.push(VecDeepPatchCommand::InsertAfter(
                                    pos,
                                    map_vec(map.next_value()?),
                                ));
                            } else if key.ends_with('<') {
                                let pos = key[..key.len() - 1].parse::<usize>().unwrap();
                                commands.push(VecDeepPatchCommand::Patch(pos, map.next_value()?));
                            } else {
                                let value: P = map.next_value()?;
                                let pos = key.parse::<usize>().unwrap();
                                commands.push(VecDeepPatchCommand::Replace(pos, value.into()));
                            }
                        }
                    },
                    StringOrNumber::Number(pos) => {
                        let value: P = map.next_value()?;
                        commands.push(VecDeepPatchCommand::Replace(pos, value.into()));
                    }
                }
            }
            Ok(commands)
        }
    }

    let visitor: VecDeepPatchCommandsVisitor<T, P> = VecDeepPatchCommandsVisitor(PhantomData);

    deserializer.deserialize_any(visitor)
}

//////////////////////// Unit tests ////////////////////////

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions_sorted::assert_eq_sorted;

    mod vec_patch {
        use super::*;
        use serde::Deserialize;
        use struct_patch::Patch;

        #[derive(Debug, Clone, PartialEq, Patch, Default)]
        #[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
        struct TestStruct {
            pub name: String,
            #[patch(name = "Option<SubTestStructPatch>")]
            pub sub: Option<SubTestStruct>,
        }

        #[derive(Debug, Clone, PartialEq, Patch, Default)]
        #[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
        struct SubTestStruct {
            #[patch(name = "VecPatch<String>")]
            pub list: Vec<String>,
            pub num: Option<u32>,
        }

        impl From<TestStructPatch> for TestStruct {
            fn from(patch: TestStructPatch) -> Self {
                let mut sub = Self::default();
                sub.apply(patch);
                sub
            }
        }

        impl From<SubTestStructPatch> for SubTestStruct {
            fn from(patch: SubTestStructPatch) -> Self {
                let mut sub = Self::default();
                sub.apply(patch);
                sub
            }
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

            let mut base_data: TestStruct = serde_yaml::from_str::<TestStructPatch>(base)
                .unwrap()
                .into();
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

            let mut base_data: TestStruct = serde_yaml::from_str::<TestStructPatch>(base)
                .unwrap()
                .into();
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

            let mut base_data: TestStruct = serde_yaml::from_str::<TestStructPatch>(base)
                .unwrap()
                .into();
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

            let mut base_data: TestStruct = serde_yaml::from_str::<TestStructPatch>(base)
                .unwrap()
                .into();
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

    mod vec_deep_patch {
        use super::*;
        use serde::Deserialize;
        use struct_patch::Patch;

        #[derive(Debug, Clone, PartialEq, Patch, Default)]
        #[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
        struct TestStruct {
            pub name: String,
            #[patch(name = "VecDeepPatch<SubTestStruct, SubTestStructPatch>")]
            pub subs: Vec<SubTestStruct>,
        }

        #[derive(Debug, Clone, PartialEq, Patch, Default)]
        #[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
        struct SubTestStruct {
            pub name: String,
            pub num: u32,
        }

        impl From<TestStructPatch> for TestStruct {
            fn from(patch: TestStructPatch) -> Self {
                let mut sub = Self::default();
                sub.apply(patch);
                sub
            }
        }

        impl From<SubTestStructPatch> for SubTestStruct {
            fn from(patch: SubTestStructPatch) -> Self {
                let mut sub = Self::default();
                sub.apply(patch);
                sub
            }
        }

        #[test]
        fn test_simple_patch() {
            let base = r#"
                name: patch1
                subs:
                  - name: sub1
                    num: 1
                  - name: sub2
                    num: 2
            "#;

            let patch = r#"
                name: patch2
            "#;

            let mut base_data: TestStruct = serde_yaml::from_str::<TestStructPatch>(base)
                .unwrap()
                .into();
            let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

            base_data.apply(patch_data);

            assert_eq_sorted!(
                base_data,
                TestStruct {
                    name: "patch2".into(),
                    subs: vec![
                        SubTestStruct {
                            name: "sub1".into(),
                            num: 1
                        },
                        SubTestStruct {
                            name: "sub2".into(),
                            num: 2
                        }
                    ],
                }
            );
        }

        #[test]
        fn test_vec_replace() {
            let base = r#"
                name: patch1
                subs:
                  - name: sub1
                    num: 1
                  - name: sub2
                    num: 2
            "#;

            let patch = r#"
                subs:
                  - name: sub3
                    num: 3
                  - name: sub4
                    num: 4
            "#;

            let mut base_data: TestStruct = serde_yaml::from_str::<TestStructPatch>(base)
                .unwrap()
                .into();
            let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

            base_data.apply(patch_data);

            assert_eq_sorted!(
                base_data,
                TestStruct {
                    name: "patch1".into(),
                    subs: vec![
                        SubTestStruct {
                            name: "sub3".into(),
                            num: 3
                        },
                        SubTestStruct {
                            name: "sub4".into(),
                            num: 4
                        }
                    ]
                }
            );
        }

        #[test]
        fn test_vec_append_patch() {
            let base = r#"
                name: patch1
                subs:
                  - name: sub1
                    num: 1
                  - name: sub2
                    num: 2
            "#;

            let patch = r#"
                subs:
                  +:
                    - name: sub3
                      num: 3
                    - name: sub4
                      num: 4
            "#;

            let mut base_data: TestStruct = serde_yaml::from_str::<TestStructPatch>(base)
                .unwrap()
                .into();
            let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

            base_data.apply(patch_data);

            assert_eq_sorted!(
                base_data,
                TestStruct {
                    name: "patch1".into(),
                    subs: vec![
                        SubTestStruct {
                            name: "sub1".into(),
                            num: 1
                        },
                        SubTestStruct {
                            name: "sub2".into(),
                            num: 2
                        },
                        SubTestStruct {
                            name: "sub3".into(),
                            num: 3
                        },
                        SubTestStruct {
                            name: "sub4".into(),
                            num: 4
                        }
                    ]
                }
            );
        }

        #[test]
        fn test_vec_replace_patch() {
            let base = r#"
                name: patch1
                subs:
                  - name: sub1
                    num: 1
                  - name: sub2
                    num: 2
            "#;

            let patch = r#"
                subs:
                  0: 
                    name: sub3
                    num: 3
            "#;

            let mut base_data: TestStruct = serde_yaml::from_str::<TestStructPatch>(base)
                .unwrap()
                .into();
            let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

            base_data.apply(patch_data);

            assert_eq_sorted!(
                base_data,
                TestStruct {
                    name: "patch1".into(),
                    subs: vec![
                        SubTestStruct {
                            name: "sub3".into(),
                            num: 3
                        },
                        SubTestStruct {
                            name: "sub2".into(),
                            num: 2
                        },
                    ]
                }
            );
        }

        #[test]
        fn test_vec_deep_patch() {
            let base = r#"
                name: patch1
                subs:
                  - name: sub1
                    num: 1
                  - name: sub2
                    num: 2
            "#;

            let patch = r#"
                subs:
                  0<: 
                    num: 3
            "#;

            let mut base_data: TestStruct = serde_yaml::from_str::<TestStructPatch>(base)
                .unwrap()
                .into();
            let patch_data: TestStructPatch = serde_yaml::from_str(patch).unwrap();

            base_data.apply(patch_data);

            assert_eq_sorted!(
                base_data,
                TestStruct {
                    name: "patch1".into(),
                    subs: vec![
                        SubTestStruct {
                            name: "sub1".into(),
                            num: 3
                        },
                        SubTestStruct {
                            name: "sub2".into(),
                            num: 2
                        },
                    ]
                }
            );
        }
    }

    #[cfg(feature = "permissive")]
    mod deserialize {
        use super::*;

        mod one_or_many {
            use super::*;

            #[derive(Deserialize, Debug, Clone, PartialEq)]
            struct TestStruct {
                pub one_or_many: OneOrMany<String>,
            }

            #[test]
            fn one() {
                let ret: TestStruct = serde_yaml::from_str("one_or_many: test").unwrap();
                assert_eq_sorted!(
                    ret,
                    TestStruct {
                        one_or_many: OneOrMany(vec!["test".into()])
                    }
                )
            }

            #[test]
            fn many() {
                let ret: TestStruct = serde_yaml::from_str("one_or_many: [test]").unwrap();
                assert_eq_sorted!(
                    ret,
                    TestStruct {
                        one_or_many: OneOrMany(vec!["test".into()])
                    }
                )
            }
        }

        mod optional_one_or_many {
            use super::*;

            #[derive(Deserialize, Debug, Clone, PartialEq, Default)]
            struct TestStruct {
                pub test: Option<String>,
                pub one_or_many: Option<OneOrMany<String>>,
            }

            #[test]
            fn one() {
                let ret: TestStruct = serde_yaml::from_str("one_or_many: test").unwrap();
                assert_eq_sorted!(
                    ret,
                    TestStruct {
                        test: None,
                        one_or_many: Some(OneOrMany(vec!["test".into()]))
                    }
                )
            }

            #[test]
            fn many() {
                let ret: TestStruct = serde_yaml::from_str("one_or_many: [test]").unwrap();
                assert_eq_sorted!(
                    ret,
                    TestStruct {
                        test: None,
                        one_or_many: Some(OneOrMany(vec!["test".into()]))
                    }
                )
            }

            #[test]
            fn null() {
                let ret: TestStruct = serde_yaml::from_str("one_or_many: null").unwrap();
                assert_eq_sorted!(
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
                assert_eq_sorted!(
                    ret,
                    TestStruct {
                        test: Some("test".into()),
                        one_or_many: None
                    }
                )
            }
        }
    }
}
