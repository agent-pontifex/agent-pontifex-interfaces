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

## Repository-local Git worktrees

- Create or use a Git worktree only when the human operator explicitly authorizes it for the current task. Concurrency or a dirty checkout is not permission by itself.
- Put every authorized worktree at `<repository-root>/tmp/worktrees/<name>`; from the repository root, use `./tmp/worktrees/<name>`. Never place worktrees beside repositories or organization directories.
- Keep `tmp`, `temp`, `tmp/worktrees`, and `temp/worktrees` ignored in the repository-root `.gitignore`. Do not commit files from those directories.
- Relocate or remove a worktree only when the operator explicitly requests it. Before removal, preserve and publish intended changes, verify its commit is represented on the target branch, and confirm there are no tracked, untracked, ignored-sensitive, or in-use files that must survive. Remove it with `git worktree remove <path>` without `--force`; never delete a worktree directory with `rm`.
