# agent-pontifex-interfaces

The language-neutral wire contract for Agent Pontifex. **Types and contracts only** — no
transport, no persistence, no provider, GitHub, Linear or Fiducia behaviour. Product
specifics belong in namespaced capabilities and extension objects.

## Two co-equal authorities

Per DEN-3959, this repository carries **two independently human-authored authorities**.
Neither is generated from the other, and neither may overwrite the other:

| Authority | Location | Downstream chain |
|---|---|---|
| TypeSpec | `typespec/main.tsp` | `IR_T → SQL_T → Protobuf/proto3 → gRPC → wire clients` |
| JSON Schema | `schemas/*.json` | `IR_J → interfaces/types/validators → SQL_J → HTTP/write clients` |

`tools/check_peer_parity.py` cross-checks them and **fails closed**. An unexplained
mismatch blocks publication, merge, release and deployment. It compares enum member sets
and model field names with optionality — the things that actually drift — and reports
anything it cannot parse as a failure rather than skipping it silently.

## Language projections

`langs/typescript/` and `langs/dart/` are **projections**, not authorities. Language
projections stay together under `langs/` so they cannot collide with authority and
package-level directories. Do not add a field to a projection that neither authority
declares.

The Rust contract already lives in
[`agent-sdk.rs/agent-pontifex-protocol`](https://github.com/agent-pontifex/agent-sdk.rs)
and is **deliberately not duplicated here**. Adding a second Rust type crate would create
two homes for the same contract — the exact defect tracked in DEN-3048. This repository is
the language-neutral authority; the Rust crate is a consumer of it.

## Invariants JSON Schema cannot express

Some protocol rules are structural and are enforced in `tools/run_conformance.py` and in
each projection's `validate()`:

- **Capabilities must be sorted.** Negotiation is deterministic; an unsorted list is invalid.
- **Capabilities and extension keys must be namespaced** (must contain a `.`).
- **`service` and `protocol` must agree** (`bridge` ⇔ `agent-pontifex.bridge`).
- **`protocol_versions.min_major ≤ max_major`**, and `min_major ≥ 1`.

## Required-but-nullable fields

`Coordinator.Job` has five fields — `claimed_by`, `lease_expires_at`, `result`,
`last_error`, `budget_usd` — that are `Option<T>` in Rust **without**
`skip_serializing_if`. They are therefore always present on the wire with a `null` value:
the **key is required, the value is nullable**. Both authorities encode this. Treating them
as optional keys is a contract break.

## Verify locally

```bash
pip install jsonschema
python3 tools/check_peer_parity.py     # authority parity, fails closed
python3 tools/run_conformance.py       # fixtures; invalid-*.json MUST be rejected
cd langs/typescript && npx tsc -p tsconfig.json --noEmit
cd ../dart && dart analyze --fatal-infos
```

Fixtures named `invalid-*.json` are expected to be **rejected**. A fixture that should fail
but passes is itself a conformance failure — that is how a weakened contract gets caught.
