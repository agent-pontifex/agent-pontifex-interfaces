#!/usr/bin/env python3
"""Validate every fixture against the JSON Schema authority, then enforce the
protocol invariants JSON Schema cannot express.

Fixtures named invalid-*.json MUST be rejected. A fixture that is supposed to
fail but passes is itself a conformance failure -- that is how a weakened
contract gets caught.
"""
from __future__ import annotations
import json, sys
from pathlib import Path
from jsonschema import Draft202012Validator, RefResolver

ROOT = Path(__file__).resolve().parent.parent
SCHEMAS = ROOT / "schemas"
store = {}
for f in SCHEMAS.glob("*.json"):
    doc = json.loads(f.read_text())
    store[doc["$id"]] = doc
    store[f.name] = doc  # relative $ref support

descriptor = store["service-descriptor.schema.json"]
resolver = RefResolver(base_uri="", referrer=descriptor, store=store)
validator = Draft202012Validator(descriptor, resolver=resolver)


def structural_invariants(doc: dict) -> list[str]:
    """Invariants enforced by agent-pontifex-protocol that JSON Schema cannot state."""
    errs = []
    caps = doc.get("capabilities", [])
    if caps != sorted(caps):
        errs.append("capabilities must be sorted for deterministic negotiation")
    pv = doc.get("protocol_versions", {})
    if pv and pv.get("min_major", 0) > pv.get("max_major", 0):
        errs.append("invalid protocol major-version range")
    return errs


failures = []
for fixture in sorted((ROOT / "conformance" / "fixtures").glob("*.json")):
    doc = json.loads(fixture.read_text())
    errs = [e.message for e in validator.iter_errors(doc)] + structural_invariants(doc)
    should_fail = fixture.name.startswith("invalid-")
    if should_fail and not errs:
        failures.append(f"{fixture.name}: expected rejection, but it validated")
    elif not should_fail and errs:
        failures.append(f"{fixture.name}: expected acceptance, got {errs}")
    else:
        verdict = "rejected (as required)" if should_fail else "accepted"
        detail = f" -- {errs[0]}" if errs else ""
        print(f"  {fixture.name}: {verdict}{detail}")

if failures:
    print("\nCONFORMANCE: FAIL")
    for f in failures:
        print(f"  - {f}")
    sys.exit(1)
print("\nCONFORMANCE: OK")
