# Repository Guidelines

Project: `D:\workspace\dark`. Rust stable, edition 2021; Windows first, Linux and macOS supported.

## Read First

- [README.md](README.md): product and usage.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): design, modules and RAM budget.
- [docs/PLANNING.md](docs/PLANNING.md): milestones and acceptance criteria.
- [docs/AGENT_WORKFLOW.md](docs/AGENT_WORKFLOW.md): communication rules, agent roles, exact models, planning approval and pipeline. These instructions apply to all work.

## Product and Code Rules

- Explain briefly and simply in Vietnamese. Code, identifiers and comments are in English; preserve Vietnamese documentation as UTF-8.
- Optimize real release-process RAM as far as possible while preserving correctness, data safety, AI quality and usability for nontechnical vibe coders. Never cut AI context to save RAM.
- Keep only essential functionality. Use one thread, lazy loading and bounded/reused buffers. Keep large data on disk and measure child processes separately.
- M0 idle memory must be below 2,000,000 bytes; the application hard limit is 10,000,000 bytes. Measure release builds in realistic scenarios, including sampled peaks.
- Use Rust naming conventions and rustfmt defaults. Keep modules aligned with the architecture.
- Add dependencies only with justification in the approved plan. M0 excludes tokio, regex, clap, serde and later-milestone features (git, diff, AI, editor).
- Preserve the release profile in Cargo.toml. Keep API keys in environment variables, never configuration or logs. Generated state belongs in ignored `.dark/`, build output in ignored `target/`.
- Plan feature work, split it into small tasks and obtain user approval before implementation. Follow an already approved plan without requesting approval again unless scope changes materially.
- Agents own separate files. Every implementation step must build before a separate focused commit. Commit messages require a purpose prefix such as `feat:`, `fix:`, `test:` or `docs:`.

## Verification

Run from the repository root:

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --release
cargo build --release
powershell -NoProfile -ExecutionPolicy Bypass -File tests/ram/windows-console.ps1
```

- Unit tests use `#[cfg(test)]`; integration tests live in `tests/`. RAM tests live under `tests/ram/` with an integration entry point. Add behavioral and regression tests for new code and fixes.
- RAM budgets apply only to release binaries; debug RAM tests must be explicitly ignored with a reason. Do not substitute test-runner memory for application memory.
- In WSL/Linux, GNU uses `CARGO_TARGET_DIR="$HOME/dark-target"`; the approved musl experiment uses `CARGO_TARGET_DIR="$HOME/dark-target-musl"` with explicit `--target x86_64-unknown-linux-musl`. Preserve GNU artifacts and the default target. For real PTY checks, pass the directory containing the chosen target's debug/release directories to `tests/ram/linux-terminal.py`. Compare VmRSS, private resident and PSS with `tests/ram/linux-memory.py`; keep VmRSS as the budget metric. Install Rust only with user authorization, through official rustup under the Linux user's home; never use sudo without explicit authorization.
- Windows uses private working set, Linux/macOS use resident memory; label the metric. Unsupported measurements are `n/a` with a reason, never another metric under the same name.
- Verify navigation, clean exit, resize and panic cleanup in actual terminal sessions. Report exact commands, results, binary size, memory measurements and unverified platforms honestly.
- Keep commits and pull requests focused; report behavior, validation and relevant milestone. Include terminal evidence for UI changes and measurements for memory-sensitive changes.
