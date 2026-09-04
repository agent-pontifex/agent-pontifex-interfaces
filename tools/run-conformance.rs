use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

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
        } else if byte == open {
            depth += 1;
            if depth == 1 {
                body_start = Some(index + 1);
            }
        } else if byte == close {
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

fn value_start(source: &str, key: &str) -> Result<usize, String> {
    let marker = format!("\"{key}\"");
    let start = source
        .find(&marker)
        .ok_or_else(|| format!("missing key {key}"))?;
    source[start + marker.len()..]
        .find(':')
        .map(|offset| start + marker.len() + offset + 1)
        .ok_or_else(|| format!("missing colon after {key}"))
}

fn json_string(source: &str, key: &str) -> Result<String, String> {
    let bytes = source.as_bytes();
    let mut index = value_start(source, key)?;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if bytes.get(index) != Some(&b'"') {
        return Err(format!("{key} must be a string"));
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
            return Ok(source[start..index].to_owned());
        }
        index += 1;
    }
    Err(format!("unterminated string for {key}"))
}

fn json_u64(source: &str, key: &str) -> Result<u64, String> {
    let bytes = source.as_bytes();
    let mut index = value_start(source, key)?;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    let start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    if start == index {
        return Err(format!("{key} must be a non-negative integer"));
    }
    source[start..index]
        .parse::<u64>()
        .map_err(|error| format!("invalid integer for {key}: {error}"))
}

fn json_string_array(source: &str, key: &str) -> Result<Vec<String>, String> {
    let start = value_start(source, key)?;
    let body = matching_body(source, start, b'[', b']')?;
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
            return Err(format!("unterminated array string for {key}"));
        }
        values.push(body[start..index].to_owned());
        index += 1;
    }
    Ok(values)
}

fn top_level_object_keys(body: &str) -> Result<BTreeSet<String>, String> {
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
                    return Err("unterminated object key".to_owned());
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

fn json_object_keys(source: &str, key: &str) -> Result<BTreeSet<String>, String> {
    let start = value_start(source, key)?;
    let body = matching_body(source, start, b'{', b'}')?;
    top_level_object_keys(&body)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|ch| ch == '_' || ch == '-' || ch == '.' || ch.is_ascii_lowercase() || ch.is_ascii_digit())
}

fn validate_descriptor(source: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let outer = matching_body(source, 0, b'{', b'}');
    let keys = outer.as_deref().and_then(top_level_object_keys);
    let required = BTreeSet::from([
        "schema_version".to_owned(),
        "protocol".to_owned(),
        "protocol_versions".to_owned(),
        "service".to_owned(),
        "implementation".to_owned(),
    ]);
    let allowed = BTreeSet::from([
        "schema_version".to_owned(),
        "protocol".to_owned(),
        "protocol_versions".to_owned(),
        "service".to_owned(),
        "implementation".to_owned(),
        "capabilities".to_owned(),
        "extensions".to_owned(),
    ]);
    match keys {
        Ok(keys) => {
            for key in required.difference(&keys) {
                errors.push(format!("missing required field {key}"));
            }
            for key in keys.difference(&allowed) {
                errors.push(format!("unknown top-level field {key}"));
            }
        }
        Err(error) => errors.push(error),
    }

    match json_u64(source, "schema_version") {
        Ok(1) => {}
        Ok(value) => errors.push(format!("unsupported schema_version {value}")),
        Err(error) => errors.push(error),
    }

    let protocol = json_string(source, "protocol");
    let service = json_string(source, "service");
    match (&service, &protocol) {
        (Ok(service), Ok(protocol)) if service == "bridge" && protocol == "agent-pontifex.bridge" => {}
        (Ok(service), Ok(protocol)) if service == "coordinator" && protocol == "agent-pontifex.coordinator" => {}
        (Ok(service), Ok(protocol)) => errors.push(format!(
            "service/protocol mismatch: service={service}, protocol={protocol}"
        )),
        (Err(error), _) | (_, Err(error)) => errors.push(error.clone()),
    }

    match json_string(source, "implementation") {
        Ok(value) if valid_identifier(&value) => {}
        Ok(value) => errors.push(format!("invalid implementation identifier {value:?}")),
        Err(error) => errors.push(error),
    }

    let versions = value_start(source, "protocol_versions")
        .and_then(|start| matching_body(source, start, b'{', b'}'));
    match versions {
        Ok(versions) => {
            let min = json_u64(&versions, "min_major");
            let max = json_u64(&versions, "max_major");
            match (min, max) {
                (Ok(min), Ok(max)) if (1..=65_535).contains(&min) && min <= max && max <= 65_535 => {}
                (Ok(min), Ok(max)) => errors.push(format!("invalid protocol range {min}..={max}")),
                (Err(error), _) | (_, Err(error)) => errors.push(error),
            }
        }
        Err(error) => errors.push(error),
    }

    match json_string_array(source, "capabilities") {
        Ok(capabilities) => {
            if capabilities.len() > 256 {
                errors.push("too many capabilities".to_owned());
            }
            let mut sorted = capabilities.clone();
            sorted.sort();
            if capabilities != sorted {
                errors.push("capabilities must be sorted for deterministic negotiation".to_owned());
            }
            let mut unique = BTreeSet::new();
            for capability in capabilities {
                if !valid_identifier(&capability) || !capability.contains('.') {
                    errors.push(format!("invalid or unnamespaced capability {capability:?}"));
                }
                if !unique.insert(capability.clone()) {
                    errors.push(format!("duplicate capability {capability:?}"));
                }
            }
        }
        Err(error) if error.starts_with("missing key capabilities") => {}
        Err(error) => errors.push(error),
    }

    match json_object_keys(source, "extensions") {
        Ok(extensions) => {
            if extensions.len() > 64 {
                errors.push("too many extensions".to_owned());
            }
            for extension in extensions {
                if !valid_identifier(&extension) || !extension.contains('.') {
                    errors.push(format!("invalid or unnamespaced extension {extension:?}"));
                }
            }
        }
        Err(error) if error.starts_with("missing key extensions") => {}
        Err(error) => errors.push(error),
    }

    errors.sort();
    errors.dedup();
    errors
}

fn run(root: &Path) -> Result<(), String> {
    let directory = root.join("conformance/fixtures");
    let mut entries = fs::read_dir(&directory)
        .map_err(|error| format!("failed reading {}: {error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed enumerating fixtures: {error}"))?;
    entries.sort_by_key(|entry| entry.file_name());

    let mut failures = Vec::new();
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "non-UTF-8 fixture name".to_owned())?;
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("failed reading {}: {error}", path.display()))?;
        let errors = validate_descriptor(&source);
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
        println!("CONFORMANCE: OK (Rust)");
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
    use super::validate_descriptor;

    const VALID: &str = r#"{
      "schema_version": 1,
      "protocol": "agent-pontifex.bridge",
      "protocol_versions": {"min_major": 1, "max_major": 1},
      "service": "bridge",
      "implementation": "agent-pontifex-community",
      "capabilities": ["bridge.agents.register", "bridge.messages.post"],
      "extensions": {"vendor.feature": {}}
    }"#;

    #[test]
    fn accepts_valid_descriptor() {
        assert!(validate_descriptor(VALID).is_empty());
    }

    #[test]
    fn rejects_protocol_mismatch() {
        let invalid = VALID.replace("agent-pontifex.bridge", "agent-pontifex.coordinator");
        assert!(validate_descriptor(&invalid).iter().any(|error| error.contains("mismatch")));
    }

    #[test]
    fn rejects_unsorted_capabilities() {
        let invalid = VALID.replace(
            "bridge.agents.register\", \"bridge.messages.post",
            "bridge.messages.post\", \"bridge.agents.register",
        );
        assert!(validate_descriptor(&invalid).iter().any(|error| error.contains("sorted")));
    }

    #[test]
    fn rejects_unnamespaced_extension() {
        let invalid = VALID.replace("vendor.feature", "feature");
        assert!(validate_descriptor(&invalid).iter().any(|error| error.contains("extension")));
    }
}
