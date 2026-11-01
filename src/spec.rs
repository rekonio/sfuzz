use std::fmt::Display;
use std::fs::File;
use std::io::{BufWriter, Write};
use oas3::OpenApiV3Spec;
use oas3::spec::{
    ParameterIn, ObjectOrReference, ObjectSchema, Schema, SchemaType as Type, SchemaTypeSet as TypeSet, Spec,
};
use serde_json::{Map, Value, json};

pub fn generate_fuzz_template(spec: &OpenApiV3Spec, out: &mut impl Write, host: Option<&str>) {
    let host = host.unwrap_or("http://localhost:8080");

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

        write!(out, "{method}").unwrap();
        let mut path = path.clone();
        for (key, value) in path_params {
            path = path.replace(&format!("{{{key}}}"), &value);
        }
        write!(out, " {host}{path}").unwrap();

        if !query_params.is_empty() {
            let query: String = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(&query_params)
                .finish();
            write!(out, "?{query}").unwrap();
        }
        if let Some(b) = body {
            write!(out, " {b}").unwrap();
        }
        writeln!(out).unwrap();
        out.flush().unwrap();
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
    let kind = match &object.schema_type {
        Some(TypeSet::Single(t)) => Some(*t),
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
        other => {
            let example = match &object.example {
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => String::new(),
            };
            let keyword = FuzzKeyword{kind: other, example, format: object.format.clone()};
            json!(format!("{keyword}"))
        }
    }
}


struct FuzzKeyword {
    kind: Option<Type>,
    format: Option<String>,
    example: String,
}

impl Display for FuzzKeyword {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let prefix = format!("FUZZ{}", self.example);
        match self.kind {
            Some(Type::String) => match self.format.as_deref() {
                Some("uuid") => write!(f, "{prefix}UID"),
                Some("date-time") => write!(f, "{prefix}DTE"),
                _ => write!(f, "{prefix}STR"),
            },
            Some(Type::Integer) | Some(Type::Number) => write!(f, "{prefix}NUM"),
            Some(Type::Boolean) => write!(f, "{prefix}BOL"),
            Some(Type::Null) | None => write!(f, "FUZZNUL"),
            _ => write!(f, "FUZZUKN"),
        }
    }
}

fn to_param_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod test {
    use std::{fs};
    use crate::spec::generate_fuzz_template;

    #[test]
    fn test_generate_fuzz_file() {
        let input = fs::read_to_string("src/testdata/basic.input.yaml").unwrap();
        let doc = oas3::from_yaml(input).expect("cannot parse spec");
        let expected = fs::read_to_string("src/testdata/basic.output.txt").unwrap();
        let mut buf = Vec::new();
        generate_fuzz_template(&doc, &mut buf, None);
        let actual = String::from_utf8(buf).unwrap();
        assert_eq!(actual, expected);
    }
}
