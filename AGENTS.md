# agent-pontifex-interfaces — agent instructions

Follow the org-wide policy in `agent-pontifex/.github` and the workspace `AGENTS.md`.
Repository-specific rules:

1. **Never generate one authority from the other.** `typespec/main.tsp` and `schemas/*.json`
   are co-equal and independently authored. A generator that overwrites either is a defect.
2. **Never add a field to a projection alone.** `langs/typescript/` and `langs/dart/`
   follow the authorities. Change the authorities first, re-run parity, then update
   projections.
3. **Do not add a Rust type crate here.** The Rust contract is
   `agent-sdk.rs/agent-pontifex-protocol`. A second home recreates DEN-3048.
4. **Types only.** No transport, persistence, provider or product behaviour, and no
   vendor-specific fields — those belong in namespaced capabilities/extensions.
5. `tools/check_peer_parity.py` and `tools/run_conformance.py` must both pass before any
   merge. They are required checks, not advisory.
