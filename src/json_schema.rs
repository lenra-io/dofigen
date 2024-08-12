///! This module provides a custom implementation of `JsonSchema`.
use schemars::{schema::*, JsonSchema};
use struct_patch::Patch;

use crate::deserialize::{VecDeepPatch, VecPatch};

#[cfg(feature = "json_schema")]
impl<T> JsonSchema for VecPatch<T>
where
    T: Clone + JsonSchema,
{
    fn schema_name() -> String {
        format!("VecPatch_for_{}", T::schema_name())
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> Schema {
        let array_schema: Schema = SchemaObject {
            array: Some(Box::new(ArrayValidation {
                items: Some(SingleOrVec::Single(Box::new(T::json_schema(generator)))),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into();

        SchemaObject {
            subschemas: Some(Box::new(SubschemaValidation {
                one_of: Some(vec![
                    #[cfg(feature = "permissive")]
                    T::json_schema(generator),
                    array_schema.clone(),
                    SchemaObject {
                        object: Some(Box::new(ObjectValidation {
                            pattern_properties: vec![
                                // ReplaceAll
                                (String::from(r"_"), array_schema.clone()),
                                // Replace
                                (String::from(r"^\d+$"), T::json_schema(generator)),
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
        format!("VecDeepPatch_for_{}", P::schema_name())
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> Schema {
        let type_schema = P::json_schema(generator);
        let array_schema: Schema = SchemaObject {
            array: Some(Box::new(ArrayValidation {
                items: Some(SingleOrVec::Single(Box::new(type_schema.clone()))),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into();

        SchemaObject {
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
