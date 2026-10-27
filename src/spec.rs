use oas3::OpenApiV3Spec;
use oas3::spec::{
    ParameterIn, ObjectOrReference, ObjectSchema, Schema, SchemaType as Type, SchemaTypeSet as TypeSet, Spec,
};
use serde_json::{Map, Value, json};

pub fn list_operations(spec: &OpenApiV3Spec) {
    for (path, method, op) in spec.operations() {
        let mut path_params: Vec<(String, String)> = vec![];
        let mut query_params: Vec<(String, String)> = vec![];
        match op.parameters(spec) {
            Ok(params) => {
                for p in &params {
                    let value = match (&p.example, &p.schema) {
                        (Some(ex), _) => ex.clone(),
                        (None, Some(schema)) => resolve_schema(schema, spec, 0),
                        (None, None) => Value::Null,
                    };
                    let pair = (p.name.clone(), to_param_string(&value));
                    match p.location {
                        ParameterIn::Path => path_params.push(pair),
                        ParameterIn::Query => query_params.push(pair),
                        ParameterIn::Header | ParameterIn::Cookie => {}
                    }
                }
            }
            Err(e) => eprintln!("{path} {method}: {e:?}"),
        }

        let body = match op.request_body(spec) {
            Ok(Some(body)) => body
                .content
                .get("application/json")
                .and_then(|media| media.schema.as_ref())
                .map(|schema| resolve_schema(schema, spec, 0)),
            Ok(None) => None,
            Err(e) => {
                eprintln!("{path} {method}: {e:?}");
                None
            }
        };

        println!("{method} {path}");
        println!("  path:  {path_params:?}");
        println!("  query: {query_params:?}");
        if let Some(b) = body {
            println!("  body:  {b}");
        }
    }
}

const MAX_RECURSIVE_SCHEMA_DEPTH: usize = 6;
fn resolve_schema(schema: &Schema, spec: &Spec, depth: usize) -> Value {
    if depth > MAX_RECURSIVE_SCHEMA_DEPTH {
        return Value::Null;
    }
    match schema.resolve(spec) {
        Ok(Schema::Object(obj)) => match *obj {
            ObjectOrReference::Object(o) => resolve_object(&o, spec, depth),
            ObjectOrReference::Ref { ref_path, .. } => {
                unreachable!("left an unresolved ref: {ref_path}")
            }
        },
        _ => Value::Null,
    }
}

fn resolve_object(object: &ObjectSchema, spec: &Spec, depth: usize) -> Value {
    if let Some(ex) = &object.example {
        return ex.clone();
    }

    let kind = match &object.schema_type {
        Some(TypeSet::Single(t)) => Some(t.clone()),
        Some(TypeSet::Multiple(ts)) => ts.first().cloned(),
        None if !object.properties.is_empty() => Some(Type::Object),
        None => None,
    };

    match kind {
        Some(Type::Object) => {
            let mut map = Map::new();
            for (name, prop) in &object.properties {
                map.insert(name.clone(), resolve_schema(prop, spec, depth + 1));
            }
            Value::Object(map)
        }
        Some(Type::Array) => match &object.items {
            Some(item) => json!([resolve_schema(item, spec, depth + 1)]),
            None => json!([]),
        },
        Some(Type::String) => match object.format.as_deref() {
            Some("uuid") => json!("deada555-beef-4000-8000-000000000000"),
            Some("date-time") => json!("2026-01-01T00:00:00Z"),
            _ => json!("happy"),
        },
        Some(Type::Integer) => json!(42),
        Some(Type::Number) => json!(42.0),
        Some(Type::Boolean) => json!(false),
        Some(Type::Null) | None => Value::Null,
    }
}

fn to_param_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
