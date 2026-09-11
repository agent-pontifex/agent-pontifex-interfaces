use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Clone, Copy)]
struct Pair {
    name: &'static str,
    schema_file: &'static str,
    definition: &'static str,
}

const ENUM_PAIRS: &[Pair] = &[
    Pair {
        name: "AgentKind",
        schema_file: "bridge.schema.json",
        definition: "agentKind",
    },
    Pair {
        name: "Role",
        schema_file: "bridge.schema.json",
        definition: "role",
    },
    Pair {
        name: "MemberRole",
        schema_file: "bridge.schema.json",
        definition: "memberRole",
    },
    Pair {
        name: "PresenceKind",
        schema_file: "bridge.schema.json",
        definition: "presenceKind",
    },
    Pair {
        name: "JobStatus",
        schema_file: "coordinator.schema.json",
        definition: "jobStatus",
    },
    Pair {
        name: "CompletionOutcome",
        schema_file: "coordinator.schema.json",
        definition: "completionOutcome",
    },
    Pair {
        name: "ServiceKind",
        schema_file: "service-descriptor.schema.json",
        definition: "serviceKind",
    },
    Pair {
        name: "MessageKind",
        schema_file: "bridge.schema.json",
        definition: "messageKind",
    },
    Pair {
        name: "DeliveryMode",
        schema_file: "bridge.schema.json",
        definition: "deliveryMode",
    },
    Pair {
        name: "AckStatus",
        schema_file: "bridge.schema.json",
        definition: "ackStatus",
    },
];

const MODEL_PAIRS: &[Pair] = &[
    Pair {
        name: "ProtocolVersionRange",
        schema_file: "common.schema.json",
        definition: "protocolVersionRange",
    },
    Pair {
        name: "ErrorResponse",
        schema_file: "common.schema.json",
        definition: "errorResponse",
    },
    Pair {
        name: "ServiceDescriptor",
        schema_file: "service-descriptor.schema.json",
        definition: "",
    },
    Pair {
        name: "Agent",
        schema_file: "bridge.schema.json",
        definition: "agent",
    },
    Pair {
        name: "FileLease",
        schema_file: "bridge.schema.json",
        definition: "fileLease",
    },
    Pair {
        name: "Message",
        schema_file: "bridge.schema.json",
        definition: "message",
    },
    Pair {
        name: "Channel",
        schema_file: "bridge.schema.json",
        definition: "channel",
    },
    Pair {
        name: "AcquireFileLeaseRequest",
        schema_file: "bridge.schema.json",
        definition: "acquireFileLeaseRequest",
    },
    Pair {
        name: "PostMessageRequest",
        schema_file: "bridge.schema.json",
        definition: "postMessageRequest",
    },
    Pair {
        name: "Job",
        schema_file: "coordinator.schema.json",
        definition: "job",
    },
    Pair {
        name: "CreateJobRequest",
        schema_file: "coordinator.schema.json",
        definition: "createJobRequest",
    },
    Pair {
        name: "ClaimJobRequest",
        schema_file: "coordinator.schema.json",
        definition: "claimJobRequest",
    },
    Pair {
        name: "CompleteJobRequest",
        schema_file: "coordinator.schema.json",
        definition: "completeJobRequest",
    },
    Pair {
        name: "AgentRef",
        schema_file: "bridge.schema.json",
        definition: "agentRef",
    },
    Pair {
        name: "TraceContext",
        schema_file: "bridge.schema.json",
        definition: "traceContext",
    },
    Pair {
        name: "LeaseRef",
        schema_file: "bridge.schema.json",
        definition: "leaseRef",
    },
    Pair {
        name: "RealtimeEnvelope",
        schema_file: "bridge.schema.json",
        definition: "realtimeEnvelope",
    },
    Pair {
        name: "Acknowledgement",
        schema_file: "bridge.schema.json",
        definition: "acknowledgement",
    },
    Pair {
        name: "WorkHandoff",
        schema_file: "bridge.schema.json",
        definition: "workHandoff",
    },
];

const SCALAR_PAIRS: &[Pair] = &[
    Pair {
        name: "Identifier",
        schema_file: "common.schema.json",
        definition: "identifier",
    },
    Pair {
        name: "NamespacedIdentifier",
        schema_file: "common.schema.json",
        definition: "namespacedIdentifier",
    },
    Pair {
        name: "Timestamp",
        schema_file: "common.schema.json",
        definition: "timestamp",
    },
];

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed reading {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("invalid JSON in {}: {error}", path.display()))
}

fn read_authored(root: &Path, file: &str) -> Result<Value, String> {
    read_json(&root.join("schemas").join(file))
}

fn definition<'a>(document: &'a Value, name: &str) -> Result<&'a Value, String> {
    if name.is_empty() {
        return Ok(document);
    }
    document
        .get("$defs")
        .and_then(Value::as_object)
        .and_then(|defs| defs.get(name))
        .ok_or_else(|| format!("missing JSON Schema definition {name}"))
}

fn ref_parts(reference: &str) -> (&str, Option<&str>) {
    reference
        .split_once('#')
        .map_or((reference, None), |(file, fragment)| (file, Some(fragment)))
}

fn resolve_ref<'a>(
    value: &'a Value,
    authored: &'a BTreeMap<String, Value>,
    generated: &'a BTreeMap<String, Value>,
) -> Result<&'a Value, String> {
    let reference = value
        .get("$ref")
        .and_then(Value::as_str)
        .ok_or_else(|| "schema reference is not a string".to_owned())?;
    let (file, fragment) = ref_parts(reference);
    if file.is_empty() {
        let name = fragment
            .and_then(|fragment| fragment.strip_prefix("/$defs/"))
            .ok_or_else(|| format!("unsupported local schema reference {reference}"))?;
        let mut matches = authored.values().filter_map(|document| {
            document
                .get("$defs")
                .and_then(Value::as_object)
                .and_then(|defs| defs.get(name))
        });
        let first = matches
            .next()
            .ok_or_else(|| format!("unresolved schema definition {reference}"))?;
        if matches.next().is_some() {
            return Err(format!(
                "ambiguous local schema definition {reference}; qualify the reference"
            ));
        }
        return Ok(first);
    }
    let document = if let Some(document) = authored.get(file) {
        Some(document)
    } else {
        let stem = Path::new(file)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(file);
        generated.get(stem)
    };
    let document = document.ok_or_else(|| format!("unresolved schema reference {reference}"))?;
    match fragment {
        None => Ok(document),
        Some(fragment) => {
            let name = fragment
                .strip_prefix("/$defs/")
                .ok_or_else(|| format!("unsupported schema fragment {fragment}"))?;
            document
                .get("$defs")
                .and_then(Value::as_object)
                .and_then(|defs| defs.get(name))
                .ok_or_else(|| format!("unresolved schema definition {reference}"))
        }
    }
}

fn canonical(value: &Value) -> String {
    serde_json::to_string(value).expect("JSON values are serializable")
}

fn enum_values(
    value: &Value,
    authored: &BTreeMap<String, Value>,
    generated: &BTreeMap<String, Value>,
) -> Result<Vec<String>, String> {
    let value = if value.get("$ref").is_some() {
        resolve_ref(value, authored, generated)?
    } else {
        value
    };
    if let Some(values) = value.get("enum").and_then(Value::as_array) {
        let mut values = values.iter().map(canonical).collect::<Vec<_>>();
        values.sort();
        return Ok(values);
    }
    if let Some(values) = value
        .get("anyOf")
        .or_else(|| value.get("oneOf"))
        .and_then(Value::as_array)
    {
        let mut result = Vec::new();
        for member in values {
            if let Some(constant) = member.get("const") {
                result.push(canonical(constant));
            } else {
                result.extend(enum_values(member, authored, generated)?);
            }
        }
        result.sort();
        return Ok(result);
    }
    Err("schema has neither enum nor const union".to_owned())
}

fn scalar_constraint(value: &Value) -> String {
    let keys = [
        "type",
        "const",
        "minLength",
        "maxLength",
        "pattern",
        "format",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "maxItems",
        "minItems",
    ];
    let mut map = Map::new();
    for key in keys {
        if let Some(item) = value.get(key) {
            map.insert(key.to_owned(), item.clone());
        }
    }
    canonical(&Value::Object(map))
}

fn shape(
    value: &Value,
    authored: &BTreeMap<String, Value>,
    generated: &BTreeMap<String, Value>,
    seen: &mut BTreeSet<String>,
) -> Result<String, String> {
    if let Some(reference) = value.get("$ref").and_then(Value::as_str) {
        let marker = reference.to_owned();
        if !seen.insert(marker.clone()) {
            return Ok("recursive".to_owned());
        }
        let resolved = resolve_ref(value, authored, generated)?;
        let result = shape(resolved, authored, generated, seen);
        seen.remove(&marker);
        return result;
    }
    if value.get("const").is_some() {
        return Ok(format!("const:{}", canonical(value.get("const").unwrap())));
    }
    if let Some(members) = value.get("allOf").and_then(Value::as_array) {
        let mut merged = Map::new();
        for member in members {
            let member = if member.get("$ref").is_some() {
                resolve_ref(member, authored, generated)?
            } else {
                member
            };
            for key in [
                "type",
                "minLength",
                "maxLength",
                "pattern",
                "format",
                "minimum",
                "maximum",
                "exclusiveMinimum",
                "exclusiveMaximum",
            ] {
                if let Some(item) = member.get(key) {
                    merged.insert(key.to_owned(), item.clone());
                }
            }
        }
        if !merged.is_empty() {
            return Ok(canonical(&Value::Object(merged)));
        }
    }
    if value.get("enum").is_some() {
        return Ok(format!(
            "enum:{:?}",
            enum_values(value, authored, generated)?
        ));
    }
    if let Some(members) = value
        .get("anyOf")
        .or_else(|| value.get("oneOf"))
        .and_then(Value::as_array)
    {
        let is_enum = members.iter().all(|member| member.get("const").is_some());
        if is_enum {
            return Ok(format!(
                "enum:{:?}",
                enum_values(value, authored, generated)?
            ));
        }
        let mut shapes = Vec::new();
        for member in members {
            shapes.push(shape(member, authored, generated, seen)?);
        }
        shapes.sort();
        shapes.dedup();
        if shapes.iter().any(|shape| shape == "any") {
            return Ok("any".to_owned());
        }
        return Ok(format!("union:{shapes:?}"));
    }
    if let Some(types) = value.get("type").and_then(Value::as_array) {
        let mut shapes = Vec::new();
        for schema_type in types {
            let member = serde_json::json!({ "type": schema_type });
            shapes.push(shape(&member, authored, generated, seen)?);
        }
        shapes.sort();
        shapes.dedup();
        return Ok(format!("union:{shapes:?}"));
    }
    if let Some(schema_type) = value.get("type") {
        if schema_type == "object" {
            let closed = value
                .get("additionalProperties")
                .and_then(Value::as_bool)
                .is_some_and(|closed| !closed)
                || value
                    .get("unevaluatedProperties")
                    .is_some_and(|properties| properties != &Value::Object(Map::new()));
            let closure = if closed { "closed" } else { "open" };
            let properties = value
                .get("properties")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            if properties.is_empty() {
                return Ok(format!("object:{closure}"));
            }
            let required: BTreeSet<String> = value
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            let mut fields = Vec::new();
            for (name, property) in properties {
                let mut nested_seen = seen.clone();
                fields.push(format!(
                    "{}:{}:{}",
                    name,
                    if required.contains(&name) {
                        "required"
                    } else {
                        "optional"
                    },
                    shape(&property, authored, generated, &mut nested_seen)?
                ));
            }
            fields.sort();
            return Ok(format!("object:{closure}{{{}}}", fields.join(",")));
        }
        if schema_type == "array" {
            let item_shape = value
                .get("items")
                .map(|items| shape(items, authored, generated, seen))
                .transpose()?
                .unwrap_or_else(|| "any".to_owned());
            let mut constraints = Vec::new();
            for key in ["minItems", "maxItems", "uniqueItems"] {
                if let Some(constraint) = value.get(key) {
                    constraints.push(format!("{key}={}", canonical(constraint)));
                }
            }
            return Ok(if constraints.is_empty() {
                format!("array<{item_shape}>")
            } else {
                format!("array<{item_shape}>;{}", constraints.join(";"))
            });
        }
        return Ok(scalar_constraint(value));
    }
    if value.as_object().is_some_and(|object| object.is_empty()) {
        return Ok("any".to_owned());
    }
    Ok(scalar_constraint(value))
}

fn authored_documents(root: &Path) -> Result<BTreeMap<String, Value>, String> {
    let mut documents = BTreeMap::new();
    for file in [
        "common.schema.json",
        "bridge.schema.json",
        "coordinator.schema.json",
        "service-descriptor.schema.json",
    ] {
        documents.insert(file.to_owned(), read_authored(root, file)?);
    }
    Ok(documents)
}

fn generated_documents(root: &Path) -> Result<BTreeMap<String, Value>, String> {
    let mut documents = BTreeMap::new();
    for entry in fs::read_dir(root).map_err(|error| {
        format!(
            "failed reading generated schema directory {}: {error}",
            root.display()
        )
    })? {
        let path = entry
            .map_err(|error| format!("failed enumerating generated schemas: {error}"))?
            .path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("generated schema has invalid filename {}", path.display()))?;
        documents.insert(name.to_owned(), read_json(&path)?);
    }
    Ok(documents)
}

fn run(root: &Path, generated_root: &Path) -> Vec<String> {
    let authored = match authored_documents(root) {
        Ok(documents) => documents,
        Err(error) => return vec![error],
    };
    let generated = match generated_documents(generated_root) {
        Ok(documents) => documents,
        Err(error) => return vec![error],
    };
    let mut findings = Vec::new();
    let expected_generated: BTreeSet<String> = ENUM_PAIRS
        .iter()
        .chain(MODEL_PAIRS)
        .chain(SCALAR_PAIRS)
        .map(|pair| pair.name.to_owned())
        .chain(["RecordUnknown".to_owned()])
        .collect();
    for name in generated.keys() {
        if !expected_generated.contains(name) {
            findings.push(format!(
                "generated TypeSpec schema {name} has no configured authored JSON Schema peer"
            ));
        }
    }

    for pair in ENUM_PAIRS {
        let authored_value = match authored
            .get(pair.schema_file)
            .and_then(|document| definition(document, pair.definition).ok())
        {
            Some(value) => value,
            None => {
                findings.push(format!("missing authored enum {}", pair.name));
                continue;
            }
        };
        let generated_value = match generated.get(pair.name) {
            Some(value) => value,
            None => continue,
        };
        let left = enum_values(authored_value, &authored, &generated);
        let right = enum_values(generated_value, &authored, &generated);
        if left != right {
            findings.push(format!("enum wire values differ for {}", pair.name));
        }
    }

    for pair in MODEL_PAIRS.iter().chain(SCALAR_PAIRS) {
        let authored_value = match authored
            .get(pair.schema_file)
            .and_then(|document| definition(document, pair.definition).ok())
        {
            Some(value) => value,
            None => {
                findings.push(format!("missing authored schema {}", pair.name));
                continue;
            }
        };
        let generated_value = match generated.get(pair.name) {
            Some(value) => value,
            None => continue,
        };
        let left = shape(authored_value, &authored, &generated, &mut BTreeSet::new());
        let right = shape(generated_value, &authored, &generated, &mut BTreeSet::new());
        if left != right {
            findings.push(format!(
                "wire shape differs for {}: authored={left:?}; generated={right:?}",
                pair.name
            ));
        }
    }

    findings.sort();
    findings.dedup();
    findings
}

fn main() -> ExitCode {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".to_owned()));
    let generated_root = PathBuf::from(
        std::env::args()
            .nth(2)
            .unwrap_or_else(|| "target/typespec-json-schema/@typespec/json-schema".to_owned()),
    );
    let findings = run(&root, &generated_root);
    if findings.is_empty() {
        println!(
            "PEER PARITY: OK ({} enums, {} models, {} scalars; authored JSON Schema A matches TypeSpec-generated JSON Schema B)",
            ENUM_PAIRS.len(),
            MODEL_PAIRS.len(),
            SCALAR_PAIRS.len()
        );
        return ExitCode::SUCCESS;
    }

    eprintln!(
        "STOPPED_FOR_EVALUATION: {} authority discrepancy(s)",
        findings.len()
    );
    for finding in findings {
        eprintln!("  - {finding}");
    }
    ExitCode::from(2)
}
