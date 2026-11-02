#!/usr/bin/env python3
"""Compare the memory use of dark with other terminal review tools on the same diff.

Linux only (uses a real PTY and /proc). Standard library only.

The script builds a synthetic git repository with a multi-file "agent" diff,
opens each tool in a pseudo-terminal, sends one navigation key, lets it idle,
and samples memory of the whole process tree (the tool and its children).

Example:
    python3 tests/bench/compare-review-tools.py \
        --dark "$HOME/dark-target-musl/x86_64-unknown-linux-musl/release/dark" \
        --tuicr "$(command -v tuicr)" \
        --hunk "$(command -v hunk)"

Reported metrics per tool (bytes):
    private  Private_Clean + Private_Dirty from smaps_rollup (memory owned by the tool)
    rss      resident set, including shared library pages
    pss      proportional set size
Only tools whose path is given are measured. Results are printed as JSON lines
followed by a median summary table. Keep the command lines below in sync with
the tools' current CLIs before publishing numbers.
"""

import argparse
import fcntl
import json
import os
import pty
import select
import shutil
import signal
import statistics
import struct
import subprocess
import sys
import tempfile
import termios
import time

FILES = 30
LINES_PER_FILE = 150


def make_repo(root):
    """Create a git repo with FILES source files, commit, then apply an 'agent' change."""

    def git(*args):
        subprocess.run(["git", *args], cwd=root, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    git("init", "-q")
    git("config", "user.email", "bench@example.invalid")
    git("config", "user.name", "bench")
    git("config", "commit.gpgsign", "false")
    for i in range(FILES):
        ext = "rs" if i % 2 == 0 else "ts"
        sub = os.path.join(root, "src" if ext == "rs" else "web")
        os.makedirs(sub, exist_ok=True)
        lines = []
        for n in range(LINES_PER_FILE):
            if ext == "rs":
                lines.append(f"fn item_{i}_{n}(value: u64) -> u64 {{ value.wrapping_mul({n + 1}) }}")
            else:
                lines.append(f"export function item{i}_{n}(value: number): number {{ return value * {n + 1}; }}")
        with open(os.path.join(sub, f"module_{i}.{ext}"), "w", encoding="utf-8") as fh:
            fh.write("\n".join(lines) + "\n")
    git("add", "-A")
    git("commit", "-qm", "base")
    changed = 0
    for dirpath, _, names in os.walk(root):
        if ".git" in dirpath.split(os.sep):
            continue
        for name in sorted(names):
            path = os.path.join(dirpath, name)
            with open(path, encoding="utf-8") as fh:
                lines = fh.read().split("\n")
            for k in range(0, len(lines), 9):
                lines[k] = lines[k] + "  // changed by agent"
            lines.insert(len(lines) // 2, "\n".join(["// new block added by agent"] * 15))
            with open(path, "w", encoding="utf-8") as fh:
                fh.write("\n".join(lines))
            changed += 1
    stat = subprocess.run(["git", "diff", "--shortstat"], cwd=root, check=True,
                          capture_output=True, text=True).stdout.strip()
    return changed, stat


def process_tree(pid):
    pending, found = [pid], []
    while pending:
        p = pending.pop()
        found.append(p)
        try:
            for task in os.listdir(f"/proc/{p}/task"):
                try:
                    with open(f"/proc/{p}/task/{task}/children") as fh:
                        pending.extend(int(x) for x in fh.read().split())
                except OSError:
                    pass
        except OSError:
            pass
    return found


def memory(pid):
    result = {"rss": 0, "private": 0, "pss": 0}
    try:
        with open(f"/proc/{pid}/smaps_rollup") as fh:
            for line in fh:
                key, *values = line.split()
                if not values:
                    continue
                if key == "Rss:":
                    result["rss"] = int(values[0]) * 1024
                elif key == "Pss:":
                    result["pss"] = int(values[0]) * 1024
                elif key in ("Private_Clean:", "Private_Dirty:"):
                    result["private"] += int(values[0]) * 1024
    except OSError:
        pass
    return result


def measure(name, argv, cwd, seconds, rows, cols):
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(cwd)
        os.environ["TERM"] = "xterm-256color"
        os.environ["COLORTERM"] = "truecolor"
        try:
            os.execvp(argv[0], argv)
        finally:
            os._exit(127)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
    samples, start, sent, output = [], time.time(), False, 0
    while time.time() - start < seconds:
        ready, _, _ = select.select([fd], [], [], 0.1)
        if ready:
            try:
                data = os.read(fd, 65536)
            except OSError:
                break
            output += len(data)
            # Answer cursor-position and device-attribute queries so TUIs do not stall.
            if b"\x1b[6n" in data:
                os.write(fd, b"\x1b[1;1R")
            if b"\x1b[c" in data or b"\x1b[0c" in data:
                os.write(fd, b"\x1b[?62;22c")
        elapsed = time.time() - start
        if elapsed > 1.0 and not sent:
            os.write(fd, b"\x1bj" if name == "dark" else b"j")  # one navigation keypress
            sent = True
        procs = process_tree(pid)
        total = {"rss": 0, "private": 0, "pss": 0}
        for p in procs:
            for key, value in memory(p).items():
                total[key] += value
        total["t"] = elapsed
        total["procs"] = len(procs)
        samples.append(total)
    try:
        os.kill(pid, signal.SIGKILL)
        os.waitpid(pid, 0)
    except OSError:
        pass
    os.close(fd)
    steady = [s for s in samples if s["t"] >= seconds - 2.0] or samples
    avg = lambda key: int(sum(s[key] for s in steady) / len(steady))
    return {
        "tool": name,
        "private": avg("private"),
        "rss": avg("rss"),
        "pss": avg("pss"),
        "peak_rss": max(s["rss"] for s in samples),
        "max_procs": max(s["procs"] for s in samples),
        "output_bytes": output,
    }


def main():
    if not sys.platform.startswith("linux"):
        sys.exit("This benchmark needs Linux (/proc and a PTY). On Windows run it in WSL.")
    if not shutil.which("git"):
        sys.exit("git is required.")
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--dark", required=True, help="path to the dark release binary")
    parser.add_argument("--dark-args", default="", help="extra arguments for dark, space separated")
    parser.add_argument("--tuicr", help="path to tuicr (runs: tuicr -w --no-update-check)")
    parser.add_argument("--hunk", help="path to hunk (runs: hunk diff)")
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--seconds", type=float, default=8.0)
    parser.add_argument("--rows", type=int, default=45)
    parser.add_argument("--cols", type=int, default=160)
    parser.add_argument("--min-output", type=int, default=200,
                        help="minimum terminal output bytes for a round to count (default 200)")
    parser.add_argument("--keep-repo", action="store_true", help="do not delete the sample repo")
    args = parser.parse_args()

    tools = {"dark": [args.dark, *args.dark_args.split()]}
    if args.tuicr:
        tools["tuicr"] = [args.tuicr, "-w", "--no-update-check"]
    if args.hunk:
        tools["hunk"] = [args.hunk, "diff"]
    for name, argv in tools.items():
        if not (os.path.isfile(argv[0]) and os.access(argv[0], os.X_OK)):
            sys.exit(f"{name}: not an executable file: {argv[0]}")

    repo = tempfile.mkdtemp(prefix="dark-bench-")
    try:
        changed, stat = make_repo(repo)
        print(json.dumps({"repo": repo, "files_changed": changed, "diff": stat,
                          "pty": f"{args.cols}x{args.rows}", "rounds": args.rounds}))
        # One unscored warm-up run per tool: first launches can be slow (caches, extraction).
        for name, argv in tools.items():
            measure(name, argv, repo, min(args.seconds, 4.0), args.rows, args.cols)
        results = {name: [] for name in tools}
        for _ in range(args.rounds):
            for name, argv in tools.items():
                r = measure(name, argv, repo, args.seconds, args.rows, args.cols)
                r["valid"] = r["output_bytes"] >= args.min_output
                results[name].append(r)
                print(json.dumps(r))
        print()
        print(f"{'tool':<8} {'private (median)':>18} {'rss (median)':>14} {'pss (median)':>14} {'peak rss':>12} {'procs':>6} {'valid':>6}")
        for name, rs in results.items():
            valid = [r for r in rs if r["valid"]]
            if not valid:
                print(f"{name:<8} no valid round: the tool produced almost no output (did it start?)")
                continue
            med = lambda key: int(statistics.median(r[key] for r in valid))
            print(f"{name:<8} {med('private'):>18,} {med('rss'):>14,} {med('pss'):>14,} "
                  f"{max(r['peak_rss'] for r in valid):>12,} {max(r['max_procs'] for r in valid):>6} "
                  f"{len(valid):>3}/{len(rs)}")
        print("\nRounds with output below --min-output bytes are excluded. Look at each tool's screen "
              "once (--keep-repo) to confirm it rendered the diff before publishing numbers.")
    finally:
        if args.keep_repo:
            print(f"sample repo kept at {repo}")
        else:
            shutil.rmtree(repo, ignore_errors=True)


if __name__ == "__main__":
    main()
