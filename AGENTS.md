# agent-pontifex-interfaces — agent instructions

Follow the org-wide policy in `agent-pontifex/.github` and the workspace `AGENTS.md`.
Repository-specific rules:

1. **Never generate one authority from the other.** `typespec/main.tsp` and `schemas/*.json`
   are co-equal and independently authored. A generator that overwrites either is a defect.
2. **Never add a field to a projection alone.** `typescript/` and `dart/` follow the
   authorities. Change both authored authorities first, re-run parity, then update projections.
3. **Do not add a Rust type crate here.** The Rust contract is
   `agent-sdk.rs/agent-pontifex-protocol`. A second home recreates DEN-3048.
4. **Types only.** No transport, persistence, provider or product behaviour, and no
   vendor-specific fields — those belong in namespaced capabilities/extensions.
5. `tools/check-peer-parity.rs` and `tools/run-conformance.rs` must compile with warnings
   denied and pass before any merge. They are required fail-closed checks, not advisory.
6. Prefer Rust for systems, parity, conformance, migration, and release tooling. Do not
   introduce Python for these paths when a dependency-free or pinned Rust implementation is
   reasonable.
7. Realtime messages are idempotent and exact-sequence aware. Any repository-writing agent
   must carry a current fencing token; stale leases are rejected rather than guessed around.
8. ChatGPT, Claude, Codex, Grok, and other providers are peers behind typed capabilities.
   Provider-specific payloads stay under namespaced extension keys and never alter the common
   envelope or authorization model.
