# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

snapdash is a **single-context** repo - one crate, one glossary, one ADR directory.

## Before exploring, read these

- **`CONTEXT.md`** at the repo root - the glossary / ubiquitous language.
- **`docs/adr/`** - read ADRs that touch the area you're about to work in.

If any of these files don't exist, **proceed silently**.
Don't flag their absence; don't suggest creating them upfront.
The `/domain-modeling` skill (reached via `/grill-with-docs` and `/improve-codebase-architecture`) creates them lazily when terms or decisions actually get resolved.

## File structure

```
/
├── CONTEXT.md
├── docs/adr/
│   ├── 0001-....md
│   └── 0002-....md
└── src/
```

If snapdash ever splits into multiple bounded contexts, switch to a root `CONTEXT-MAP.md` pointing at per-context `CONTEXT.md` files under `src/<context>/`, each with its own `docs/adr/` for context-scoped decisions, and update this file.

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `CONTEXT.md`.
Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal - either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007 (widget size model) - but worth reopening because…_
