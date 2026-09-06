# Peer-authority canary

This directory contains two independently authored, intentionally minimal contract authorities used only to prove that the fleet parity gate is installed and executable:

- `main.tsp` is the TypeSpec authority for this canary.
- `authored.schema.json` is the JSON Schema Draft 2020-12 authority for this canary.

The pinned validator generates JSON Schema B from TypeSpec only as comparison evidence under `.typespec-json-schema-validator/`. It must never overwrite either authored source.

A passing canary certifies the workflow and pinned toolchain, not the repository's product contracts. Product declarations remain fail-closed until their independently authored TypeSpec and JSON Schema lanes converge under DEN-3043.
