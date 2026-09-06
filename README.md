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

TypeSpec emits a temporary JSON Schema B under `target/typespec-json-schema/`; that output
is never committed and never overwrites JSON Schema A. The Rust gates independently
validate fixtures against authored JSON Schema A and generated B, then require matching
accept/reject verdicts. `tools/check-peer-parity.rs` also compares configured enums,
requiredness, wire types, scalar constraints, array bounds, unions, and nested property
shapes. An unexplained mismatch blocks publication, merge, release and deployment.

## Language projections

`typescript/` and `dart/` are **projections**, not authorities. Do not add a field to a
projection that neither authority declares.

The Rust contract already lives in
[`agent-sdk.rs/agent-pontifex-protocol`](https://github.com/agent-pontifex/agent-sdk.rs)
and is **deliberately not duplicated here**. Adding a second Rust type crate would create
two homes for the same contract — the exact defect tracked in DEN-3048. This repository is
the language-neutral authority; the Rust crate is a consumer of it.

## Invariants JSON Schema cannot express

Some protocol rules are structural and are enforced in `tools/run-conformance.rs` and in
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
npm ci --prefix typespec --ignore-scripts --no-audit --no-fund
typespec/node_modules/.bin/tsp compile typespec/main.tsp --no-emit
typespec/node_modules/.bin/tsp compile typespec/main.tsp \
  --emit @typespec/json-schema \
  --option @typespec/json-schema.emitAllModels=true \
  --option @typespec/json-schema.file-type=json \
  --option @typespec/json-schema.int64-strategy=number \
  --option @typespec/json-schema.seal-object-schemas=true \
  --output-dir target/typespec-json-schema
cargo test --locked --all-targets
cargo run --locked --bin check-peer-parity -- \
  . target/typespec-json-schema/@typespec/json-schema
cargo run --locked --bin run-conformance -- \
  . target/typespec-json-schema/@typespec/json-schema
npx --yes --package typescript@5.9.2 tsc -p typescript/tsconfig.json --noEmit
dart pub get --directory dart
dart analyze --fatal-infos dart
```

Fixtures named `invalid-*.json` are expected to be **rejected**. A fixture that should fail
but passes is itself a conformance failure — that is how a weakened contract gets caught.
