# Repository Guidelines

Rust stable, edition 2021. Read [README.md](README.md), [architecture](docs/ARCHITECTURE.md), [milestones](docs/PLANNING.md), [agent workflow](docs/AGENT_WORKFLOW.md) and [development instructions](docs/DEVELOPMENT.md).

## Rules

- Explain briefly and simply in Vietnamese; use English for code, identifiers and comments. Preserve UTF-8 documentation.
- Serve nontechnical vibe coders: keep essential features and simple workflows. Preserve correctness, data safety and AI quality; never cut AI context to save RAM.
- Scope: seven tabs (Review, Edit, Vibe, Git commit, Models, View, Help) as listed in docs/FEATURES.md; milestones and phases in docs/PLANNING.md. A lightweight editor and file tree are in scope; no IDE features (autocomplete, multi-cursor, file tabs, vim mode, plugins), repo map, LSP, advanced git (rebase, merge, push) or embedded agent TUI; see docs/FEATURES.md §6.
- UI must match docs/UI_SPEC.md and the reference images in docs/ui/ (layout, colors, borders, key hints). All colors come from the single theme module (src/ui/theme.rs) with the exact hex values in UI_SPEC §2; never hard-code colors elsewhere. Report any intentional difference from the images. Change UI_SPEC before changing the UI design.
- Minimize real release-process RAM: one thread, lazy loading, reused buffers and large data on disk. Measure child processes separately.
- RAM budgets per milestone are in docs/PLANNING.md and docs/ARCHITECTURE.md §6 (review idle below 2,000,000 bytes; hard limit 10,000,000 bytes). Agent processes are measured and reported separately. Do not relax or substitute metrics to report a pass.
- Follow rustfmt and Rust naming conventions. Justify dependencies in an approved plan; preserve the release profile. Do not implement features of later milestones.
- Plan feature work and obtain approval before implementation; follow approved work without asking again unless scope changes materially. Agents own separate files; roles/models and pipeline are defined in the agent workflow.
- Add behavioral/regression tests. Every implementation step must build before a focused commit with a prefix such as feat:, fix:, test: or docs:.
- Keep API keys in environment variables, never configuration or logs. Keep scripts tracked in scripts/ or tests/; .dark/ contains generated output only. Report actual checks, memory, binary size and unverified platforms honestly.

## Checks

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --release
cargo build --release
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-windows.ps1
bash scripts/verify-linux.sh musl
```
