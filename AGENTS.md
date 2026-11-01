# Repository Guidelines

Rust stable, edition 2021. Read [README.md](README.md), [architecture](docs/ARCHITECTURE.md), [milestones](docs/PLANNING.md), [agent workflow](docs/AGENT_WORKFLOW.md) and [development instructions](docs/DEVELOPMENT.md).

## Rules

- Explain briefly and simply in Vietnamese; use English for code, identifiers and comments. Preserve UTF-8 documentation.
- Serve nontechnical vibe coders: keep essential features and simple workflows. Preserve correctness, data safety and AI quality; never cut AI context to save RAM.
- Minimize real release-process RAM: one thread, lazy loading, reused buffers and large data on disk. Measure child processes separately.
- M0 idle budget is strictly below 2,000,000 bytes; the application hard limit is 10,000,000 bytes. Do not relax or substitute metrics to report a pass.
- Follow rustfmt and Rust naming conventions. Justify dependencies in an approved plan; preserve the release profile. Do not implement later milestones during M0.
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
