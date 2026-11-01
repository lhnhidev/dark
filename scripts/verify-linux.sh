#!/usr/bin/env bash
set -uo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
export PATH="$HOME/.cargo/bin:$PATH"
variant="${1:-musl}"
case "$variant" in
    musl)
        export CARGO_TARGET_DIR="$HOME/dark-target-musl"
        target=(--target x86_64-unknown-linux-musl)
        binaries="$CARGO_TARGET_DIR/x86_64-unknown-linux-musl"
        ;;
    gnu)
        export CARGO_TARGET_DIR="$HOME/dark-target"
        target=()
        binaries="$CARGO_TARGET_DIR"
        ;;
    *) echo 'Usage: bash scripts/verify-linux.sh [musl|gnu] [OUTPUT_DIRECTORY]' >&2; exit 2 ;;
esac
output="${2:-$repo_root/.dark/verification/m0-review/after/$variant}"
if ! mkdir -p "$output"; then echo "Cannot create log directory: $output" >&2; exit 1; fi
failed=0
run() {
    local name="$1"
    shift
    if ! { printf '$'; printf ' %q' "$@"; printf '\n'; } | tee "$output/$name.txt"; then
        echo "Cannot write log: $output/$name.txt" >&2
        failed=1
        return
    fi
    "$@" 2>&1 | tee -a "$output/$name.txt"
    local pipeline_status=("${PIPESTATUS[@]}")
    local code=${pipeline_status[0]}
    local log_code=${pipeline_status[1]}
    if ! printf 'exit_code=%s\nlog_exit_code=%s\n' "$code" "$log_code" | tee -a "$output/$name.txt"; then
        echo "Cannot finish log: $output/$name.txt" >&2
        failed=1
    fi
    if ((code != 0 || log_code != 0)); then failed=1; fi
}
run environment bash -c 'id -un; uname -srm; echo "CARGO_TARGET_DIR=$CARGO_TARGET_DIR"; command -v cargo; cargo --version; rustc --version; command -v cc'
run fmt cargo fmt --check
run clippy cargo clippy --locked --all-targets "${target[@]}" -- -D warnings
run test cargo test --locked "${target[@]}"
run test-release cargo test --locked --release "${target[@]}" -- --nocapture
run build-debug cargo build --locked "${target[@]}"
run build-release cargo build --locked --release "${target[@]}"
run self-test "$binaries/release/dark" --self-test
run help "$binaries/release/dark" --help
run version "$binaries/release/dark" --version
run binary-size stat --format=%s "$binaries/release/dark"
if [[ -f "$binaries/release/dark" ]] && (( $(stat --format=%s "$binaries/release/dark") >= 3000000 )); then failed=1; fi
run terminal python3 tests/ram/linux-terminal.py "$binaries"
exit "$failed"
