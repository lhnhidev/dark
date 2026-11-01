#!/usr/bin/env bash
# Run explicitly only after authorizing a Rust installation in this Linux user account.
set -euo pipefail
if [[ "$(uname -s)" != Linux ]]; then echo 'Run this script inside Linux/WSL.' >&2; exit 2; fi
if ! command -v cc; then
    echo 'Missing C linker. Run yourself: sudo apt update && sudo apt install build-essential' >&2
    exit 2
fi
if ! command -v curl; then echo 'curl is required; install it before running this script.' >&2; exit 2; fi
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output="$repo_root/.dark/verification/setup"
mkdir -p "$output"
export CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME="$HOME/.rustup"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable --component rustfmt --component clippy --no-modify-path 2>&1 | tee "$output/rustup.txt"
export PATH="$CARGO_HOME/bin:$PATH"
cargo --version
rustc --version
