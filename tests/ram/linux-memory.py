#!/usr/bin/env python3
"""Compare actual GNU and musl TUI memory: linux-memory.py GNU_BINARY MUSL_BINARY."""

import fcntl
import json
import os
import pty
import resource
import select
import signal
import struct
import subprocess
import sys
import termios
import time


def size(fd, rows, cols):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


def read_fields(path, keys):
    found = {}
    with open(path, encoding="ascii") as source:
        for line in source:
            fields = line.split()
            if fields and fields[0].endswith(":") and fields[0][:-1] in keys:
                if len(fields) != 3 or fields[2] != "kB":
                    raise RuntimeError(f"invalid memory field in {path}: {line!r}")
                found[fields[0][:-1]] = int(fields[1]) * 1024
    missing = keys - found.keys()
    if missing:
        raise RuntimeError(f"missing {sorted(missing)} in {path}")
    return found


def measure(pid):
    status = read_fields(f"/proc/{pid}/status", {"VmRSS"})
    rollup = read_fields(f"/proc/{pid}/smaps_rollup", {
        "Rss", "Pss", "Private_Clean", "Private_Dirty", "Shared_Clean", "Shared_Dirty"
    })
    if not status["VmRSS"] or not rollup["Rss"]:
        raise RuntimeError("RSS unavailable or zero; cannot verify memory")
    return {"vmrss_bytes": status["VmRSS"], "rollup_rss_bytes": rollup["Rss"],
            "private_resident_bytes": rollup["Private_Clean"] + rollup["Private_Dirty"],
            "pss_bytes": rollup["Pss"],
            "shared_resident_bytes": rollup["Shared_Clean"] + rollup["Shared_Dirty"]}


def launch(binary, debug):
    master, slave = pty.openpty()
    child = None
    output = bytearray()
    peak = 0
    try:
        size(slave, 24, 80)
        before = termios.tcgetattr(slave)

        def setup():
            resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
            os.setsid()
            fcntl.ioctl(slave, termios.TIOCSCTTY, 0)

        env = os.environ.copy()
        env["TERM"] = "xterm-256color"
        child = subprocess.Popen([binary] + (["--debug"] if debug else []),
                                 stdin=slave, stdout=slave, stderr=slave,
                                 env=env, preexec_fn=setup, close_fds=True)

        def sample():
            nonlocal peak
            metrics = measure(child.pid)
            peak = max(peak, metrics["vmrss_bytes"])
            return metrics

        def wait_for(text, start):
            deadline = time.monotonic() + 8
            while text not in output[start:]:
                if child.poll() is not None:
                    raise RuntimeError(f"application exited before {text!r}")
                if time.monotonic() >= deadline:
                    raise RuntimeError(f"timed out waiting for {text!r}")
                if select.select([master], [], [], 0.02)[0]:
                    output.extend(os.read(master, 65536))
                sample()

        wait_for(b"[1 Review]", 0)
        for key, expected in ((b"2", b"[2 Edit]"), (b"3", b"[3 Models]"),
                              (b"1", b"[1 Review]")):
            start = len(output)
            os.write(master, key)
            wait_for(expected, start)
        start = len(output)
        size(slave, 12, 60)
        os.kill(child.pid, signal.SIGWINCH)
        wait_for(b"[1 Review]", start)
        start = len(output)
        size(slave, 24, 80)
        os.kill(child.pid, signal.SIGWINCH)
        wait_for(b"[1 Review]", start)
        # Let both variants settle equally, including at least one debug refresh.
        deadline = time.monotonic() + 1.2
        while time.monotonic() < deadline:
            if child.poll() is not None:
                raise RuntimeError("application exited during warmup")
            if select.select([master], [], [], 0.02)[0]:
                output.extend(os.read(master, 65536))
            sample()
        samples = []
        for _ in range(5):
            time.sleep(0.1)
            samples.append(sample())
        pid = child.pid
        os.write(master, b"q")
        child.wait(timeout=8)
        if child.returncode != 0:
            raise RuntimeError(f"q exit failed: {child.returncode}")
        if termios.tcgetattr(slave) != before:
            raise RuntimeError("terminal attributes were not restored")
        return {"pid": pid, "warmup_seconds_min": 1.2, "samples": samples, "peak_sampled_vmrss_bytes": peak,
                "idle_budget_pass": max(x["vmrss_bytes"] for x in samples) < 2_000_000,
                "peak_budget_pass": peak < 10_000_000}
    finally:
        if child is not None and child.poll() is None:
            child.kill()
            child.wait()
        os.close(master)
        os.close(slave)


def main():
    if not sys.platform.startswith("linux") or len(sys.argv) != 3:
        raise SystemExit("Usage on Linux: linux-memory.py GNU_BINARY MUSL_BINARY")
    failed = False
    metrics = ("vmrss_bytes", "rollup_rss_bytes", "private_resident_bytes",
               "pss_bytes", "shared_resident_bytes")
    for name, binary in zip(("gnu", "musl"), sys.argv[1:]):
        for debug in (False, True):
            variant = f"{name}{'-debug' if debug else '-normal'}"
            launches = []
            for index in range(5):
                try:
                    result = launch(binary, debug)
                    launches.append(result)
                    print(json.dumps({"variant": variant, "launch": index + 1, **result}), flush=True)
                except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
                    failed = True
                    print(json.dumps({"variant": variant, "launch": index + 1,
                                      "error": str(error)}), flush=True)
            if not launches:
                print(json.dumps({"variant": variant, "summary": "unavailable"}), flush=True)
                continue
            summary = {}
            summary["warmup_seconds_min"] = 1.2
            for metric in metrics:
                values = [sample[metric] for run in launches for sample in run["samples"]]
                summary[metric] = {"min": min(values), "max": max(values)}
            summary["peak_sampled_vmrss_max_bytes"] = max(r["peak_sampled_vmrss_bytes"] for r in launches)
            summary["idle_budget_pass"] = len(launches) == 5 and all(r["idle_budget_pass"] for r in launches)
            summary["peak_budget_pass"] = len(launches) == 5 and all(r["peak_budget_pass"] for r in launches)
            failed |= not summary["idle_budget_pass"] or not summary["peak_budget_pass"]
            print(json.dumps({"variant": variant, "summary": summary}), flush=True)
    if failed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
