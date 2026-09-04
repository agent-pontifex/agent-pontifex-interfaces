use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

#[derive(Clone, Copy)]
struct EnumPair {
    type_spec: &'static str,
    schema_file: &'static str,
    schema_definition: &'static str,
}

#[derive(Clone, Copy)]
struct ModelPair {
    type_spec: &'static str,
    schema_file: &'static str,
    schema_definition: &'static str,
}

const ENUM_PAIRS: &[EnumPair] = &[
    EnumPair { type_spec: "AgentKind", schema_file: "bridge.schema.json", schema_definition: "agentKind" },
    EnumPair { type_spec: "Role", schema_file: "bridge.schema.json", schema_definition: "role" },
    EnumPair { type_spec: "MemberRole", schema_file: "bridge.schema.json", schema_definition: "memberRole" },
    EnumPair { type_spec: "PresenceKind", schema_file: "bridge.schema.json", schema_definition: "presenceKind" },
    EnumPair { type_spec: "JobStatus", schema_file: "coordinator.schema.json", schema_definition: "jobStatus" },
    EnumPair { type_spec: "CompletionOutcome", schema_file: "coordinator.schema.json", schema_definition: "completionOutcome" },
    EnumPair { type_spec: "ServiceKind", schema_file: "service-descriptor.schema.json", schema_definition: "serviceKind" },
    EnumPair { type_spec: "MessageKind", schema_file: "bridge.schema.json", schema_definition: "messageKind" },
    EnumPair { type_spec: "DeliveryMode", schema_file: "bridge.schema.json", schema_definition: "deliveryMode" },
    EnumPair { type_spec: "AckStatus", schema_file: "bridge.schema.json", schema_definition: "ackStatus" },
];

const MODEL_PAIRS: &[ModelPair] = &[
    ModelPair { type_spec: "ProtocolVersionRange", schema_file: "common.schema.json", schema_definition: "protocolVersionRange" },
    ModelPair { type_spec: "ErrorResponse", schema_file: "common.schema.json", schema_definition: "errorResponse" },
    ModelPair { type_spec: "ServiceDescriptor", schema_file: "service-descriptor.schema.json", schema_definition: "" },
    ModelPair { type_spec: "Agent", schema_file: "bridge.schema.json", schema_definition: "agent" },
    ModelPair { type_spec: "FileLease", schema_file: "bridge.schema.json", schema_definition: "fileLease" },
    ModelPair { type_spec: "Message", schema_file: "bridge.schema.json", schema_definition: "message" },
    ModelPair { type_spec: "Channel", schema_file: "bridge.schema.json", schema_definition: "channel" },
    ModelPair { type_spec: "AcquireFileLeaseRequest", schema_file: "bridge.schema.json", schema_definition: "acquireFileLeaseRequest" },
    ModelPair { type_spec: "PostMessageRequest", schema_file: "bridge.schema.json", schema_definition: "postMessageRequest" },
    ModelPair { type_spec: "Job", schema_file: "coordinator.schema.json", schema_definition: "job" },
    ModelPair { type_spec: "CreateJobRequest", schema_file: "coordinator.schema.json", schema_definition: "createJobRequest" },
    ModelPair { type_spec: "ClaimJobRequest", schema_file: "coordinator.schema.json", schema_definition: "claimJobRequest" },
    ModelPair { type_spec: "CompleteJobRequest", schema_file: "coordinator.schema.json", schema_definition: "completeJobRequest" },
    ModelPair { type_spec: "AgentRef", schema_file: "bridge.schema.json", schema_definition: "agentRef" },
    ModelPair { type_spec: "TraceContext", schema_file: "bridge.schema.json", schema_definition: "traceContext" },
    ModelPair { type_spec: "LeaseRef", schema_file: "bridge.schema.json", schema_definition: "leaseRef" },
    ModelPair { type_spec: "RealtimeEnvelope", schema_file: "bridge.schema.json", schema_definition: "realtimeEnvelope" },
    ModelPair { type_spec: "Acknowledgement", schema_file: "bridge.schema.json", schema_definition: "acknowledgement" },
    ModelPair { type_spec: "WorkHandoff", schema_file: "bridge.schema.json", schema_definition: "workHandoff" },
];

fn matching_body(source: &str, search_from: usize, open: u8, close: u8) -> Result<String, String> {
    let bytes = source.as_bytes();
    let start = bytes
        .iter()
        .enumerate()
        .skip(search_from)
        .find_map(|(index, byte)| (*byte == open).then_some(index))
        .ok_or_else(|| format!("missing delimiter {}", open as char))?;

    let mut depth = 0usize;
    let mut body_start = None;
    let mut in_string = false;
    let mut escaped = false;

    for (index, byte) in bytes.iter().copied().enumerate().skip(start) {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        if byte == b'"' {
            in_string = true;
            continue;
        }
        if byte == open {
            depth += 1;
            if depth == 1 {
                body_start = Some(index + 1);
            }
            continue;
        }
        if byte == close {
            if depth == 0 {
                return Err("unbalanced closing delimiter".to_owned());
            }
            depth -= 1;
            if depth == 0 {
                let body_start = body_start.ok_or_else(|| "missing body start".to_owned())?;
                return Ok(source[body_start..index].to_owned());
            }
        }
    }

    Err("unterminated delimited body".to_owned())
}

fn type_spec_body(source: &str, keyword: &str, name: &str) -> Result<String, String> {
    let marker = format!("{keyword} {name}");
    let start = source
        .find(&marker)
        .ok_or_else(|| format!("missing TypeSpec {keyword} {name}"))?;
    matching_body(source, start + marker.len(), b'{', b'}')
}

fn strip_line_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(prefix, _)| prefix)
}

fn type_spec_enum(source: &str, name: &str) -> Result<Vec<String>, String> {
    let body = type_spec_body(source, "enum", name)?;
    let compact = body
        .lines()
        .map(strip_line_comment)
        .collect::<Vec<_>>()
        .join(" ");
    let mut values = Vec::new();
    let mut seen = BTreeSet::new();

    for raw in compact.split(',') {
        let member = raw.trim();
        if member.is_empty() {
            continue;
        }
        let value = if let Some((_, wire)) = member.split_once(':') {
            wire.trim().trim_matches('"').to_owned()
        } else {
            member
                .split_whitespace()
                .next()
                .ok_or_else(|| format!("empty TypeSpec enum member in {name}"))?
                .to_owned()
        };
        if !seen.insert(value.clone()) {
            return Err(format!("duplicate TypeSpec enum value in {name}: {value}"));
        }
        values.push(value);
    }

    Ok(values)
}

fn remove_decorators(mut line: &str) -> Result<&str, String> {
    loop {
        line = line.trim_start();
        if !line.starts_with('@') {
            return Ok(line);
        }
        let open = line
            .find('(')
            .ok_or_else(|| format!("unsupported decorator syntax: {line}"))?;
        let mut depth = 0usize;
        let mut end = None;
        let mut in_string = false;
        let mut escaped = false;
        for (offset, byte) in line.as_bytes().iter().copied().enumerate().skip(open) {
            if in_string {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    in_string = false;
                }
                continue;
            }
            if byte == b'"' {
                in_string = true;
            } else if byte == b'(' {
                depth += 1;
            } else if byte == b')' {
                if depth == 0 {
                    return Err(format!("unbalanced decorator: {line}"));
                }
                depth -= 1;
                if depth == 0 {
                    end = Some(offset + 1);
                    break;
                }
            }
        }
        let end = end.ok_or_else(|| format!("unterminated decorator: {line}"))?;
        line = &line[end..];
    }
}

fn type_spec_model(source: &str, name: &str) -> Result<BTreeMap<String, bool>, String> {
    let body = type_spec_body(source, "model", name)?;
    let mut fields = BTreeMap::new();

    for raw in body.lines() {
        let line = remove_decorators(strip_line_comment(raw).trim())?.trim();
        if line.is_empty() {
            continue;
        }
        let Some((left, _)) = line.split_once(':') else {
            return Err(format!("unsupported TypeSpec model line in {name}: {line}"));
        };
        let field = left.trim();
        let optional = field.ends_with('?');
        let field = field.trim_end_matches('?').trim();
        if field.is_empty() || !field.chars().all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
            return Err(format!("invalid TypeSpec field in {name}: {field}"));
        }
        if fields.insert(field.to_owned(), optional).is_some() {
            return Err(format!("duplicate TypeSpec field in {name}: {field}"));
        }
    }

    Ok(fields)
}

fn json_definition(source: &str, name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Ok(source.to_owned());
    }
    let marker = format!("\"{name}\"");
    let start = source
        .find(&marker)
        .ok_or_else(|| format!("missing JSON Schema definition {name}"))?;
    matching_body(source, start + marker.len(), b'{', b'}')
}

fn json_value_body(source: &str, key: &str, open: u8, close: u8) -> Result<String, String> {
    let marker = format!("\"{key}\"");
    let start = source
        .find(&marker)
        .ok_or_else(|| format!("missing JSON key {key}"))?;
    let colon = source[start + marker.len()..]
        .find(':')
        .map(|offset| start + marker.len() + offset + 1)
        .ok_or_else(|| format!("missing colon after JSON key {key}"))?;
    matching_body(source, colon, open, close)
}

fn json_string_array(source: &str, key: &str) -> Result<Vec<String>, String> {
    let body = json_value_body(source, key, b'[', b']')?;
    let bytes = body.as_bytes();
    let mut values = Vec::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }
        index += 1;
        let start = index;
        let mut escaped = false;
        while index < bytes.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                break;
            }
            index += 1;
        }
        if index >= bytes.len() {
            return Err(format!("unterminated JSON string in {key}"));
        }
        values.push(body[start..index].to_owned());
        index += 1;
    }

    Ok(values)
}

fn json_object_keys(source: &str, key: &str) -> Result<BTreeSet<String>, String> {
    let body = json_value_body(source, key, b'{', b'}')?;
    let bytes = body.as_bytes();
    let mut keys = BTreeSet::new();
    let mut index = 0usize;
    let mut object_depth = 0usize;
    let mut array_depth = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'{' => {
                object_depth += 1;
                index += 1;
            }
            b'}' => {
                object_depth = object_depth.saturating_sub(1);
                index += 1;
            }
            b'[' => {
                array_depth += 1;
                index += 1;
            }
            b']' => {
                array_depth = array_depth.saturating_sub(1);
                index += 1;
            }
            b'"' => {
                index += 1;
                let start = index;
                let mut escaped = false;
                while index < bytes.len() {
                    let byte = bytes[index];
                    if escaped {
                        escaped = false;
                    } else if byte == b'\\' {
                        escaped = true;
                    } else if byte == b'"' {
                        break;
                    }
                    index += 1;
                }
                if index >= bytes.len() {
                    return Err(format!("unterminated JSON object key in {key}"));
                }
                let candidate = &body[start..index];
                index += 1;
                let mut cursor = index;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if object_depth == 0 && array_depth == 0 && bytes.get(cursor) == Some(&b':') {
                    keys.insert(candidate.to_owned());
                }
            }
            _ => index += 1,
        }
    }

    Ok(keys)
}

fn json_model(source: &str, definition: &str) -> Result<BTreeMap<String, bool>, String> {
    let definition = json_definition(source, definition)?;
    let properties = json_object_keys(&definition, "properties")?;
    let required: BTreeSet<String> = json_string_array(&definition, "required")?.into_iter().collect();

    for field in &required {
        if !properties.contains(field) {
            return Err(format!("required JSON Schema field is absent from properties: {field}"));
        }
    }

    Ok(properties
        .into_iter()
        .map(|field| {
            let optional = !required.contains(&field);
            (field, optional)
        })
        .collect())
}

fn declared_type_spec_symbols(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let rest = line
                .strip_prefix("enum ")
                .or_else(|| line.strip_prefix("model "))?;
            rest.split(|ch: char| ch.is_ascii_whitespace() || ch == '{')
                .find(|part| !part.is_empty())
                .map(str::to_owned)
        })
        .collect()
}

fn read_schema(root: &Path, file: &str) -> Result<String, String> {
    fs::read_to_string(root.join("schemas").join(file))
        .map_err(|error| format!("failed reading schemas/{file}: {error}"))
}

fn run(root: &Path) -> Vec<String> {
    let type_spec = match fs::read_to_string(root.join("typespec/main.tsp")) {
        Ok(source) => source,
        Err(error) => return vec![format!("failed reading TypeSpec authority: {error}")],
    };
    let mut findings = Vec::new();

    for pair in ENUM_PAIRS {
        let schema = match read_schema(root, pair.schema_file) {
            Ok(schema) => schema,
            Err(error) => {
                findings.push(error);
                continue;
            }
        };
        let type_spec_values = type_spec_enum(&type_spec, pair.type_spec);
        let schema_values = json_definition(&schema, pair.schema_definition)
            .and_then(|definition| json_string_array(&definition, "enum"));
        match (type_spec_values, schema_values) {
            (Ok(left), Ok(right)) if left == right => {}
            (Ok(left), Ok(right)) => findings.push(format!(
                "enum {} differs: TypeSpec={left:?}; JSON Schema={right:?}",
                pair.type_spec
            )),
            (Err(error), _) | (_, Err(error)) => findings.push(error),
        }
    }

    for pair in MODEL_PAIRS {
        let schema = match read_schema(root, pair.schema_file) {
            Ok(schema) => schema,
            Err(error) => {
                findings.push(error);
                continue;
            }
        };
        let type_spec_fields = type_spec_model(&type_spec, pair.type_spec);
        let schema_fields = json_model(&schema, pair.schema_definition);
        match (type_spec_fields, schema_fields) {
            (Ok(left), Ok(right)) if left == right => {}
            (Ok(left), Ok(right)) => findings.push(format!(
                "model {} differs: TypeSpec={left:?}; JSON Schema={right:?}",
                pair.type_spec
            )),
            (Err(error), _) | (_, Err(error)) => findings.push(error),
        }
    }

    let covered: BTreeSet<String> = ENUM_PAIRS
        .iter()
        .map(|pair| pair.type_spec.to_owned())
        .chain(MODEL_PAIRS.iter().map(|pair| pair.type_spec.to_owned()))
        .collect();
    let uncovered: Vec<String> = declared_type_spec_symbols(&type_spec)
        .difference(&covered)
        .cloned()
        .collect();
    if !uncovered.is_empty() {
        findings.push(format!(
            "TypeSpec symbols have no configured JSON Schema peer: {uncovered:?}"
        ));
    }

    findings.sort();
    findings.dedup();
    findings
}

fn main() -> ExitCode {
    let root = std::env::args().nth(1).unwrap_or_else(|| ".".to_owned());
    let findings = run(Path::new(&root));
    if findings.is_empty() {
        println!(
            "PEER PARITY: OK ({} enums, {} models cross-checked in Rust)",
            ENUM_PAIRS.len(),
            MODEL_PAIRS.len()
        );
        return ExitCode::SUCCESS;
    }

    eprintln!("STOPPED_FOR_EVALUATION: {} peer-authority discrepancy(s)", findings.len());
    for finding in findings {
        eprintln!("  - {finding}");
    }
    ExitCode::from(2)
}
