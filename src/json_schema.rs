use crate::deserialize::*;
///! This module provides a custom implementation of `JsonSchema`.
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use std::{borrow::Cow, str::FromStr};
use struct_patch::Patch;

#[cfg(feature = "permissive")]
impl<T> JsonSchema for ParsableStruct<T>
where
    T: Clone + JsonSchema + FromStr,
{
    fn schema_id() -> Cow<'static, str> {
        format!("ParsableStruct<{}>", T::schema_name()).into()
    }

    fn schema_name() -> Cow<'static, str> {
        format!("ParsableStruct_{}", T::schema_name()).into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "title": Self::schema_id(),
            "oneOf": [
                generator.subschema_for::<T>(),
                generator.subschema_for::<String>(),
            ]
        })
    }
}

#[cfg(feature = "permissive")]
impl<T> JsonSchema for OneOrMany<T>
where
    T: Clone + JsonSchema,
{
    fn schema_id() -> Cow<'static, str> {
        format!("OneOrMany<{}>", T::schema_name()).into()
    }
    fn schema_name() -> Cow<'static, str> {
        format!("OneOrMany_{}", T::schema_name()).into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "title": Self::schema_id(),
            "oneOf": [
                generator.subschema_for::<T>(),
                {
                    "type": "array",
                    "items": generator.subschema_for::<T>()
                }
            ]
        })
    }
}

impl<T> JsonSchema for VecPatch<T>
where
    T: Clone + JsonSchema,
{
    fn schema_id() -> Cow<'static, str> {
        format!("VecPatch<{}>", T::schema_name()).into()
    }
    fn schema_name() -> Cow<'static, str> {
        format!("VecPatch_{}", T::schema_name()).into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let type_schema = generator.subschema_for::<T>();
        let array_schema = json_schema!({
            "type": "array",
            "items": type_schema
        });
        let patterns_schema = json_schema!({
            "type": "object",
            "patternProperties": {
                // ReplaceAll
                r"_": array_schema,
                // Replace
                r"^\d+$": type_schema,
                // InsertBefore
                r"^\+\d+$": array_schema,
                // InsertAfter
                r"^\d+\+$": array_schema,
                // Append
                r"^\+$": array_schema,
            }
        });

        let one_of = vec![
            #[cfg(feature = "permissive")]
            type_schema,
            array_schema,
            patterns_schema,
        ];

        json_schema!({
            "title": Self::schema_id(),
            "oneOf": one_of
        })
    }
}

impl<T, P> JsonSchema for VecDeepPatch<T, P>
where
    T: Clone + Patch<P> + From<P>,
    P: Clone + JsonSchema,
{
    fn schema_id() -> Cow<'static, str> {
        format!("VecDeepPatch<{}>", P::schema_name()).into()
    }
    fn schema_name() -> Cow<'static, str> {
        format!("VecDeepPatch_{}", P::schema_name()).into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let type_schema = generator.subschema_for::<P>();
        let array_schema = json_schema!({
            "type": "array",
            "items": type_schema
        });
        let patterns_schema = json_schema!({
            "type": "object",
            "patternProperties": {
                // ReplaceAll
                r"_": array_schema,
                // Replace
                r"^\d+$": type_schema,
                // Patch
                r"^\d+<$": type_schema,
                // InsertBefore
                r"^\+\d+$": array_schema,
                // InsertAfter
                r"^\d+\+$": array_schema,
                // Append
                r"^\+$": array_schema,
            }
        });

        let one_of = vec![
            #[cfg(feature = "permissive")]
            type_schema,
            array_schema,
            patterns_schema,
        ];

        json_schema!({
            "title": Self::schema_id(),
            "oneOf": one_of
        })
    }
}

impl<K, V> JsonSchema for HashMapPatch<K, V>
where
    K: Clone + Eq + std::hash::Hash + JsonSchema,
    V: Clone + JsonSchema,
{
    fn schema_id() -> Cow<'static, str> {
        format!("HashMapPatch<{}, {}>", K::schema_name(), V::schema_name()).into()
    }
    fn schema_name() -> Cow<'static, str> {
        format!("HashMapPatch_{}_{}", K::schema_name(), V::schema_name()).into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "title": Self::schema_id(),
            "type": "object",
            "patternProperties": {
                "^.+$": generator.subschema_for::<Option<V>>()
            }
        })
    }
}

impl<K, V> JsonSchema for HashMapDeepPatch<K, V>
where
    K: Clone + Eq + std::hash::Hash + JsonSchema,
    V: Clone + JsonSchema,
{
    fn schema_id() -> Cow<'static, str> {
        format!(
            "HashMapDeepPatch<{}, {}>",
            K::schema_name(),
            V::schema_name()
        )
        .into()
    }
    fn schema_name() -> Cow<'static, str> {
        format!("HashMapDeepPatch_{}_{}", K::schema_name(), V::schema_name()).into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "title": Self::schema_id(),
            "type": "object",
            "patternProperties": {
                "^.+$": generator.subschema_for::<Option<V>>()
            }
        })
    }
}
