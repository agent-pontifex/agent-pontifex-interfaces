use jsonschema::{Draft, Registry};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SCHEMA_BASE: &str = "https://agent-pontifex.github.io/schemas/";

struct UniqueJson(Value);

struct UniqueJsonVisitor;

impl<'de> Visitor<'de> for UniqueJsonVisitor {
    type Value = UniqueJson;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(UniqueJson(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(UniqueJson(Value::Number(value.into())))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(UniqueJson(Value::Number(value.into())))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(|number| UniqueJson(Value::Number(number)))
            .ok_or_else(|| E::custom("non-finite numbers are not valid JSON"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(UniqueJson(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(UniqueJson(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(UniqueJson(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(UniqueJson(Value::Null))
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(UniqueJson(value)) = access.next_element()? {
            values.push(value);
        }
        Ok(UniqueJson(Value::Array(values)))
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        let mut values = serde_json::Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object key {key:?}"
                )));
            }
            let UniqueJson(value) = access.next_value()?;
            values.insert(key, value);
        }
        Ok(UniqueJson(Value::Object(values)))
    }
}

impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueJsonVisitor)
    }
}

fn parse_json_text(text: &str) -> Result<Value, String> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let UniqueJson(value) = UniqueJson::deserialize(&mut deserializer)
        .map_err(|error| format!("invalid JSON: {error}"))?;
    deserializer
        .end()
        .map_err(|error| format!("trailing JSON data: {error}"))?;
    Ok(value)
}

fn parse_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed reading {}: {error}", path.display()))?;
    parse_json_text(&text).map_err(|error| format!("{}: {error}", path.display()))
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("schema field {field:?} must be a string"))
}

fn load_schema_resources(root: &Path) -> Result<Vec<(String, Value)>, String> {
    let schema_directory = root.join("schemas");
    let mut paths = fs::read_dir(&schema_directory)
        .map_err(|error| format!("failed reading {}: {error}", schema_directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed enumerating schemas: {error}"))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<PathBuf>>();
    paths.sort();

    let mut resources = Vec::new();
    for path in paths {
        let schema = parse_json(&path)?;
        if !jsonschema::draft202012::meta::is_valid(&schema) {
            return Err(format!(
                "{} is not a valid Draft 2020-12 schema",
                path.display()
            ));
        }
        let id = string_field(&schema, "$id")?.to_owned();
        resources.push((id, schema));
    }
    Ok(resources)
}

fn build_registry(resources: &[(String, Value)]) -> Result<Registry<'_>, String> {
    let mut registry = Registry::new();
    for (id, schema) in resources {
        registry = registry
            .add(id.as_str(), schema)
            .map_err(|error| format!("failed registering {id}: {error}"))?;
    }
    registry
        .prepare()
        .map_err(|error| format!("failed preparing schema registry: {error}"))
}

fn valid_namespaced_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.contains('.')
        && value.chars().all(|character| {
            character == '_'
                || character == '-'
                || character == '.'
                || character.is_ascii_lowercase()
                || character.is_ascii_digit()
        })
}

fn descriptor_invariants(document: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let service = document.get("service").and_then(Value::as_str);
    let protocol = document.get("protocol").and_then(Value::as_str);
    match (service, protocol) {
        (Some("bridge"), Some("agent-pontifex.bridge"))
        | (Some("coordinator"), Some("agent-pontifex.coordinator")) => {}
        (Some(service), Some(protocol)) => errors.push(format!(
            "service/protocol mismatch: service={service}, protocol={protocol}"
        )),
        _ => {}
    }
    let capabilities = document.get("capabilities").and_then(Value::as_array);
    if let Some(capabilities) = capabilities {
        if capabilities.len() > 256 {
            errors.push("too many capabilities".to_owned());
        }
        if capabilities
            .windows(2)
            .any(|pair| pair[0].as_str().unwrap_or_default() > pair[1].as_str().unwrap_or_default())
        {
            errors.push("capabilities must be sorted for deterministic negotiation".to_owned());
        }
        let mut unique = BTreeSet::new();
        for capability in capabilities {
            let value = capability.as_str().unwrap_or_default();
            if !valid_namespaced_identifier(value) {
                errors.push(format!("invalid or unnamespaced capability {value:?}"));
            }
            if !unique.insert(value.to_owned()) {
                errors.push(format!("duplicate capability {value:?}"));
            }
        }
    }
    if let Some(extensions) = document.get("extensions").and_then(Value::as_object) {
        if extensions.len() > 64 {
            errors.push("too many extensions".to_owned());
        }
        for key in extensions.keys() {
            if !valid_namespaced_identifier(key) {
                errors.push(format!("invalid or unnamespaced extension {key:?}"));
            }
        }
    }
    let versions = document.get("protocol_versions").and_then(Value::as_object);
    if let Some(versions) = versions {
        let min = versions.get("min_major").and_then(Value::as_u64);
        let max = versions.get("max_major").and_then(Value::as_u64);
        if let (Some(min), Some(max)) = (min, max) {
            if min > max {
                errors.push("invalid protocol major-version range".to_owned());
            }
        }
    }
    errors
}

fn realtime_invariants(document: &Value, kind: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if kind == "realtime-envelope" {
        if let Some(recipients) = document.get("recipients").and_then(Value::as_array) {
            let mut identities = BTreeSet::new();
            for recipient in recipients {
                let key = recipient.get("agent_key").and_then(Value::as_str);
                let instance = recipient.get("instance_id").and_then(Value::as_str);
                if let (Some(key), Some(instance)) = (key, instance) {
                    if !identities.insert((key.to_owned(), instance.to_owned())) {
                        errors.push(format!("duplicate recipient identity {key}/{instance}"));
                    }
                }
            }
        }
    }
    if kind == "acknowledgement" {
        let status = document.get("status").and_then(Value::as_str);
        let reason_required = matches!(
            status,
            Some("rejected" | "expired" | "unauthorized" | "stale_lease")
        );
        if reason_required
            && document
                .get("reason_code")
                .and_then(Value::as_str)
                .is_none()
        {
            errors.push("acknowledgements for rejected, expired, unauthorized, or stale leases require reason_code".to_owned());
        }
    }
    errors
}

fn fixture_definition(name: &str) -> Option<(&'static str, &'static str)> {
    if name.contains("realtime-envelope") {
        Some(("bridge.schema.json", "realtimeEnvelope"))
    } else if name.contains("acknowledgement") {
        Some(("bridge.schema.json", "acknowledgement"))
    } else if name.contains("work-handoff") {
        Some(("bridge.schema.json", "workHandoff"))
    } else {
        None
    }
}

fn semantic_kind(name: &str) -> &'static str {
    if name.contains("realtime-envelope") {
        "realtime-envelope"
    } else if name.contains("acknowledgement") {
        "acknowledgement"
    } else if name.contains("work-handoff") {
        "work-handoff"
    } else {
        "service-descriptor"
    }
}

fn validate_fixture(
    document: &Value,
    name: &str,
    resources: &[(String, Value)],
    registry: &Registry<'_>,
) -> Result<Vec<String>, String> {
    let semantic_kind = semantic_kind(name);
    let schema = if let Some((schema, definition)) = fixture_definition(name) {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": format!("{SCHEMA_BASE}{name}.fixture.json"),
            "$ref": format!("{schema}#/$defs/{definition}"),
        })
    } else {
        resources
            .iter()
            .find(|(id, _)| id.ends_with("service-descriptor.schema.json"))
            .map(|(_, value)| value.clone())
            .ok_or_else(|| "service-descriptor.schema.json is missing".to_owned())?
    };

    let validator = jsonschema::options()
        .with_draft(Draft::Draft202012)
        .with_registry(registry)
        .with_base_uri(SCHEMA_BASE)
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("invalid validator for {name}: {error}"))?;
    let mut errors = validator
        .iter_errors(document)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    if semantic_kind == "service-descriptor" {
        errors.extend(descriptor_invariants(document));
    } else {
        errors.extend(realtime_invariants(document, semantic_kind));
    }
    Ok(errors)
}

fn load_generated_resources(root: &Path) -> Result<Vec<(String, Value)>, String> {
    let mut paths = fs::read_dir(root)
        .map_err(|error| {
            format!(
                "failed reading generated TypeSpec schemas {}: {error}",
                root.display()
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed enumerating generated TypeSpec schemas: {error}"))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<PathBuf>>();
    paths.sort();

    let mut resources = Vec::new();
    for path in paths {
        let mut schema = parse_json(&path)?;
        if !jsonschema::draft202012::meta::is_valid(&schema) {
            return Err(format!(
                "generated {} is not a valid Draft 2020-12 schema",
                path.display()
            ));
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("non-UTF-8 generated schema name: {}", path.display()))?;
        let id = format!("{SCHEMA_BASE}{file_name}");
        schema
            .as_object_mut()
            .ok_or_else(|| format!("generated {file_name} must be an object schema"))?
            .insert("$id".to_owned(), Value::String(id.clone()));
        resources.push((id, schema));
    }
    Ok(resources)
}

fn generated_file_for_fixture(name: &str) -> &'static str {
    if name.contains("realtime-envelope") {
        "RealtimeEnvelope.json"
    } else if name.contains("acknowledgement") {
        "Acknowledgement.json"
    } else if name.contains("work-handoff") {
        "WorkHandoff.json"
    } else {
        "ServiceDescriptor.json"
    }
}

fn schema_instance_errors(
    document: &Value,
    schema: &Value,
    registry: &Registry<'_>,
) -> Result<Vec<String>, String> {
    let validator = jsonschema::options()
        .with_draft(Draft::Draft202012)
        .with_registry(registry)
        .with_base_uri(SCHEMA_BASE)
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("generated schema could not compile: {error}"))?;
    Ok(validator
        .iter_errors(document)
        .map(|error| error.to_string())
        .collect())
}

fn validate_generated_projection(
    root: &Path,
    generated_root: &Path,
    authored_resources: &[(String, Value)],
    authored_registry: &Registry<'_>,
) -> Result<(), String> {
    let generated_resources = load_generated_resources(generated_root)?;
    let generated_registry = build_registry(&generated_resources)?;
    let fixture_directory = root.join("conformance/fixtures");
    let mut failures = Vec::new();
    let mut checked = 0usize;

    for entry in fs::read_dir(&fixture_directory)
        .map_err(|error| format!("failed reading {}: {error}", fixture_directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("failed reading conformance fixture entry: {error}"))?
            .path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("non-UTF-8 fixture name: {}", path.display()))?;
        let document = match parse_json(&path) {
            Ok(document) => document,
            Err(_) => continue,
        };
        let generated_file = generated_file_for_fixture(name);
        let generated_schema = generated_resources
            .iter()
            .find(|(id, _)| id.ends_with(generated_file))
            .map(|(_, schema)| schema)
            .ok_or_else(|| format!("generated TypeSpec schema {generated_file} is missing"))?;
        let authored_errors =
            validate_fixture(&document, name, authored_resources, authored_registry)?;
        let mut generated_errors =
            schema_instance_errors(&document, generated_schema, &generated_registry)?;
        let semantic_kind = semantic_kind(name);
        if semantic_kind == "service-descriptor" {
            generated_errors.extend(descriptor_invariants(&document));
        } else {
            generated_errors.extend(realtime_invariants(&document, semantic_kind));
        }
        if authored_errors.is_empty() != generated_errors.is_empty() {
            failures.push(format!(
                "{name}: independently authored JSON Schema and generated TypeSpec schema disagree; authored={authored_errors:?}, generated={generated_errors:?}"
            ));
        }
        checked += 1;
    }

    if failures.is_empty() {
        println!(
            "TYPE_SPEC_JSON_SCHEMA_PARITY: OK ({checked} fixtures validated by both authorities)"
        );
        Ok(())
    } else {
        Err(failures.join("\n  - "))
    }
}

fn run(root: &Path) -> Result<(), String> {
    let resources = load_schema_resources(root)?;
    let registry = build_registry(&resources)?;
    let fixture_directory = root.join("conformance/fixtures");
    let mut paths = fs::read_dir(&fixture_directory)
        .map_err(|error| format!("failed reading {}: {error}", fixture_directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed enumerating fixtures: {error}"))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<PathBuf>>();
    paths.sort();

    let mut failures = Vec::new();
    for path in paths {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("non-UTF-8 fixture name: {}", path.display()))?;
        let document = match parse_json(&path) {
            Ok(document) => document,
            Err(error) => {
                if name.starts_with("invalid-") {
                    println!("  {name}: rejected (as required) -- {error}");
                    continue;
                }
                failures.push(format!("{name}: {error}"));
                continue;
            }
        };
        let errors = validate_fixture(&document, name, &resources, &registry)?;
        let should_fail = name.starts_with("invalid-");
        if should_fail && errors.is_empty() {
            failures.push(format!("{name}: expected rejection, but it validated"));
        } else if !should_fail && !errors.is_empty() {
            failures.push(format!("{name}: expected acceptance, got {errors:?}"));
        } else if should_fail {
            println!("  {name}: rejected (as required) -- {}", errors[0]);
        } else {
            println!("  {name}: accepted");
        }
    }

    if failures.is_empty() {
        println!("CONFORMANCE: OK (Rust JSON Schema validator)");
        if let Some(generated_root) = std::env::args().nth(2) {
            validate_generated_projection(root, Path::new(&generated_root), &resources, &registry)?;
        }
        Ok(())
    } else {
        Err(failures.join("\n  - "))
    }
}

fn main() -> ExitCode {
    let root = std::env::args().nth(1).unwrap_or_else(|| ".".to_owned());
    match run(Path::new(&root)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("CONFORMANCE: FAIL\n  - {error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_json_text, realtime_invariants};

    #[test]
    fn rejects_duplicate_object_keys() {
        let result = parse_json_text(r#"{"schema_version": 1, "schema_version": 2}"#);
        assert!(result.is_err(), "duplicate keys must fail closed");
    }

    #[test]
    fn rejects_duplicate_realtime_recipient_identity() {
        let document = serde_json::json!({
            "recipients": [
                {"agent_key": "agent.one", "instance_id": "one"},
                {"agent_key": "agent.one", "instance_id": "one"}
            ]
        });
        assert!(!realtime_invariants(&document, "realtime-envelope").is_empty());
    }

    #[test]
    fn requires_reason_for_stale_lease_acknowledgement() {
        let document = serde_json::json!({"status": "stale_lease"});
        assert!(!realtime_invariants(&document, "acknowledgement").is_empty());
    }
}
