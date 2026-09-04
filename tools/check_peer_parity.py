#!/usr/bin/env python3
"""Fail-closed cross-check between the two co-equal contract authorities.

TypeSpec (typespec/main.tsp) and JSON Schema (schemas/*.json) are independently
authored. Neither is generated from the other. This checker proves they still
describe the same wire contract, and exits non-zero on any divergence it cannot
explain. Per DEN-3959, an unexplained mismatch blocks publication, merge,
release and deployment.

It deliberately does NOT try to be a TypeSpec compiler. It extracts the facts
that actually drift in practice -- enum member sets and model field names with
their optionality -- and compares those. Anything it cannot parse is reported
as a failure, never silently skipped.
"""
from __future__ import annotations
import json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FAILURES: list[str] = []


def fail(msg: str) -> None:
    FAILURES.append(msg)


def load_schema(name: str) -> dict:
    return json.loads((ROOT / "schemas" / name).read_text())


# ---------------------------------------------------------------- TypeSpec side
TSP = (ROOT / "typespec" / "main.tsp").read_text()


def tsp_enums() -> dict[str, set[str]]:
    out = {}
    for m in re.finditer(r"\benum\s+(\w+)\s*\{([^}]*)\}", TSP):
        name, body = m.group(1), m.group(2)
        members = {t.strip() for t in body.split(",") if t.strip()}
        out[name] = members
    return out


def tsp_models() -> dict[str, dict[str, bool]]:
    """model name -> {field: is_optional}"""
    out: dict[str, dict[str, bool]] = {}
    for m in re.finditer(r"\bmodel\s+(\w+)\s*\{", TSP):
        name = m.group(1)
        depth, i = 1, m.end()
        while i < len(TSP) and depth:
            if TSP[i] == "{":
                depth += 1
            elif TSP[i] == "}":
                depth -= 1
            i += 1
        body = TSP[m.end(): i - 1]
        fields: dict[str, bool] = {}
        for line in body.splitlines():
            line = line.split("//")[0].strip()
            if not line or line.startswith("@") and ":" not in line:
                continue
            line = re.sub(r"^(@\w+\([^)]*\)\s*)+", "", line).strip()
            fm = re.match(r"(\w+)(\??)\s*:", line)
            if fm:
                fields[fm.group(1)] = fm.group(2) == "?"
        out[name] = fields
    return out


# -------------------------------------------------------------- JSON Schema side
def schema_fields(node: dict) -> dict[str, bool]:
    props = node.get("properties", {})
    required = set(node.get("required", []))
    return {k: (k not in required) for k in props}


def check_enum(tsp_name: str, schema_file: str, pointer: str) -> None:
    enums = tsp_enums()
    if tsp_name not in enums:
        return fail(f"TypeSpec enum {tsp_name} not found")
    node = load_schema(schema_file)
    for part in pointer.split("/"):
        node = node[part]
    js = set(node.get("enum", []))
    if js != enums[tsp_name]:
        fail(f"enum {tsp_name}: TypeSpec={sorted(enums[tsp_name])} JSONSchema={sorted(js)}")


def check_model(tsp_name: str, schema_file: str, pointer: str) -> None:
    models = tsp_models()
    if tsp_name not in models:
        return fail(f"TypeSpec model {tsp_name} not found")
    node = load_schema(schema_file)
    for part in pointer.split("/"):
        if part:
            node = node[part]
    jf = schema_fields(node)
    tf = models[tsp_name]
    if set(tf) != set(jf):
        only_t, only_j = sorted(set(tf) - set(jf)), sorted(set(jf) - set(tf))
        fail(f"model {tsp_name}: fields only in TypeSpec={only_t} only in JSONSchema={only_j}")
        return
    for field in sorted(tf):
        if tf[field] != jf[field]:
            fail(f"model {tsp_name}.{field}: optional in TypeSpec={tf[field]} in JSONSchema={jf[field]}")


PAIRS_ENUM = [
    ("AgentKind", "bridge.schema.json", "$defs/agentKind"),
    ("Role", "bridge.schema.json", "$defs/role"),
    ("MemberRole", "bridge.schema.json", "$defs/memberRole"),
    ("PresenceKind", "bridge.schema.json", "$defs/presenceKind"),
    ("JobStatus", "coordinator.schema.json", "$defs/jobStatus"),
    ("CompletionOutcome", "coordinator.schema.json", "$defs/completionOutcome"),
    ("ServiceKind", "service-descriptor.schema.json", "$defs/serviceKind"),
]
PAIRS_MODEL = [
    ("ProtocolVersionRange", "common.schema.json", "$defs/protocolVersionRange"),
    ("ErrorResponse", "common.schema.json", "$defs/errorResponse"),
    ("ServiceDescriptor", "service-descriptor.schema.json", ""),
    ("Agent", "bridge.schema.json", "$defs/agent"),
    ("FileLease", "bridge.schema.json", "$defs/fileLease"),
    ("Message", "bridge.schema.json", "$defs/message"),
    ("Channel", "bridge.schema.json", "$defs/channel"),
    ("AcquireFileLeaseRequest", "bridge.schema.json", "$defs/acquireFileLeaseRequest"),
    ("PostMessageRequest", "bridge.schema.json", "$defs/postMessageRequest"),
    ("Job", "coordinator.schema.json", "$defs/job"),
    ("CreateJobRequest", "coordinator.schema.json", "$defs/createJobRequest"),
    ("ClaimJobRequest", "coordinator.schema.json", "$defs/claimJobRequest"),
    ("CompleteJobRequest", "coordinator.schema.json", "$defs/completeJobRequest"),
]

for a, b, c in PAIRS_ENUM:
    check_enum(a, b, c)
for a, b, c in PAIRS_MODEL:
    check_model(a, b, c)

covered = {n for n, _, _ in PAIRS_MODEL} | {n for n, _, _ in PAIRS_ENUM}
declared = set(tsp_models()) | set(tsp_enums())
uncovered = declared - covered
if uncovered:
    fail(f"TypeSpec declares {sorted(uncovered)} with no JSON Schema peer -- add the peer or the pair table entry")

if FAILURES:
    print("PEER PARITY: FAIL")
    for f in FAILURES:
        print(f"  - {f}")
    sys.exit(1)
print(f"PEER PARITY: OK ({len(PAIRS_ENUM)} enums, {len(PAIRS_MODEL)} models cross-checked)")
