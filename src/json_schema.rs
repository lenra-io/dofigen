use crate::deserialize::*;
///! This module provides a custom implementation of `JsonSchema`.
use schemars::{schema::*, JsonSchema};
#[cfg(feature = "permissive")]
use std::str::FromStr;
use struct_patch::Patch;

#[cfg(feature = "permissive")]
impl<T> JsonSchema for ParsableStruct<T>
where
    T: Clone + JsonSchema + FromStr,
{
    fn schema_name() -> String {
        format!("ParsableStruct<{}>", T::schema_name())
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> Schema {
        SchemaObject {
            metadata: Some(Box::new(Metadata {
                title: Some(Self::schema_name()),
                ..Default::default()
            })),
            subschemas: Some(Box::new(SubschemaValidation {
                one_of: Some(vec![
                    generator.subschema_for::<T>(),
                    String::json_schema(generator),
                ]),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

#[cfg(feature = "permissive")]
impl<T> JsonSchema for OneOrMany<T>
where
    T: Clone + JsonSchema,
{
    fn schema_name() -> String {
        format!("OneOrMany<{}>", T::schema_name())
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> Schema {
        let type_ref: Schema = generator.subschema_for::<T>();
        SchemaObject {
            metadata: Some(Box::new(Metadata {
                title: Some(Self::schema_name()),
                ..Default::default()
            })),
            subschemas: Some(Box::new(SubschemaValidation {
                one_of: Some(vec![
                    type_ref.clone(),
                    SchemaObject {
                        array: Some(Box::new(ArrayValidation {
                            items: Some(SingleOrVec::Single(Box::new(type_ref))),
                            ..Default::default()
                        })),
                        ..Default::default()
                    }
                    .into(),
                ]),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

impl<T> JsonSchema for VecPatch<T>
where
    T: Clone + JsonSchema,
{
    fn schema_name() -> String {
        format!("VecPatch<{}>", T::schema_name())
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> Schema {
        let type_ref: Schema = generator.subschema_for::<T>();
        let array_schema: Schema = SchemaObject {
            array: Some(Box::new(ArrayValidation {
                items: Some(SingleOrVec::Single(Box::new(type_ref.clone()))),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into();

        SchemaObject {
            metadata: Some(Box::new(Metadata {
                title: Some(Self::schema_name()),
                ..Default::default()
            })),
            subschemas: Some(Box::new(SubschemaValidation {
                one_of: Some(vec![
                    #[cfg(feature = "permissive")]
                    type_ref.clone(),
                    array_schema.clone(),
                    SchemaObject {
                        object: Some(Box::new(ObjectValidation {
                            pattern_properties: vec![
                                // ReplaceAll
                                (String::from(r"_"), array_schema.clone()),
                                // Replace
                                (String::from(r"^\d+$"), type_ref),
                                // InsertBefore
                                (String::from(r"^\+\d+$"), array_schema.clone()),
                                // InsertAfter
                                (String::from(r"^\d+\+$"), array_schema.clone()),
                                // Append
                                (String::from(r"^\+$"), array_schema),
                            ]
                            .into_iter()
                            .collect(),
                            ..Default::default()
                        })),
                        ..Default::default()
                    }
                    .into(),
                ]),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

impl<T, P> JsonSchema for VecDeepPatch<T, P>
where
    T: Clone + Patch<P> + From<P>,
    P: Clone + JsonSchema,
{
    fn schema_name() -> String {
        format!("VecDeepPatch<{}>", P::schema_name())
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> Schema {
        let type_schema: Schema = generator.subschema_for::<P>();
        let array_schema: Schema = SchemaObject {
            array: Some(Box::new(ArrayValidation {
                items: Some(SingleOrVec::Single(Box::new(type_schema.clone()))),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into();

        SchemaObject {
            metadata: Some(Box::new(Metadata {
                title: Some(Self::schema_name()),
                ..Default::default()
            })),
            subschemas: Some(Box::new(SubschemaValidation {
                one_of: Some(vec![
                    #[cfg(feature = "permissive")]
                    type_schema.clone(),
                    array_schema.clone(),
                    SchemaObject {
                        object: Some(Box::new(ObjectValidation {
                            pattern_properties: vec![
                                // ReplaceAll
                                (String::from(r"_"), array_schema.clone()),
                                // Replace
                                (String::from(r"^\d+$"), type_schema.clone()),
                                // Patch
                                (String::from(r"^\d+<$"), type_schema.clone()),
                                // InsertBefore
                                (String::from(r"^\+\d+$"), array_schema.clone()),
                                // InsertAfter
                                (String::from(r"^\d+\+$"), array_schema.clone()),
                                // Append
                                (String::from(r"^\+$"), array_schema),
                            ]
                            .into_iter()
                            .collect(),
                            ..Default::default()
                        })),
                        ..Default::default()
                    }
                    .into(),
                ]),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

impl<K, V> JsonSchema for HashMapPatch<K, V>
where
    K: Clone + Eq + std::hash::Hash + JsonSchema,
    V: Clone + JsonSchema,
{
    fn schema_name() -> String {
        format!("HashMapPatch<{}, {}>", K::schema_name(), V::schema_name())
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> Schema {
        SchemaObject {
            metadata: Some(Box::new(Metadata {
                title: Some(Self::schema_name()),
                ..Default::default()
            })),
            object: Some(Box::new(ObjectValidation {
                pattern_properties: vec![(
                    String::from(r"^.+$"),
                    generator.subschema_for::<Option<V>>(),
                )]
                .into_iter()
                .collect(),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

impl<K, V> JsonSchema for HashMapDeepPatch<K, V>
where
    K: Clone + Eq + std::hash::Hash + JsonSchema,
    V: Clone + JsonSchema,
{
    fn schema_name() -> String {
        format!(
            "HashMapDeepPatch<{}, {}>",
            K::schema_name(),
            V::schema_name()
        )
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> Schema {
        SchemaObject {
            metadata: Some(Box::new(Metadata {
                title: Some(Self::schema_name()),
                ..Default::default()
            })),
            object: Some(Box::new(ObjectValidation {
                pattern_properties: vec![(
                    String::from(r"^.+$"),
                    generator.subschema_for::<Option<V>>(),
                )]
                .into_iter()
                .collect(),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}
