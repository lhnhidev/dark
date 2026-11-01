#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
case "${1:-musl}" in
    musl) binary="$HOME/dark-target-musl/x86_64-unknown-linux-musl/release/dark" ;;
    gnu) binary="$HOME/dark-target/release/dark" ;;
    *) echo 'Usage: bash scripts/linux-interactive.sh [musl|gnu]' >&2; exit 2 ;;
esac
export TERM="${TERM:-xterm-256color}"
output="$repo_root/.dark/verification/manual"
mkdir -p "$output"
tty | tee "$output/tty.txt"
echo $$ > "$output/pid.txt"
ulimit -c 0
exec "$binary" --debug
