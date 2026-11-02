#!/usr/bin/env python3
"""Exercise real Linux PTYs. Usage: python3 tests/ram/linux-terminal.py TARGET_DIR"""

import fcntl
import os
import pty
import re
import resource
import select
import signal
import struct
import subprocess
import sys
import termios
import tempfile
import time

IDLE_LIMIT = 2_000_000
PEAK_LIMIT = 10_000_000
TIMEOUT = 30.0


def resize(fd, rows, cols):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


def rss(pid):
    with open(f"/proc/{pid}/status", encoding="ascii") as status:
        for line in status:
            if line.startswith("VmRSS:"):
                fields = line.split()
                if len(fields) != 3 or fields[2] != "kB":
                    raise RuntimeError(f"unexpected VmRSS: {line!r}")
                value = int(fields[1]) * 1024
                if value <= 0:
                    raise RuntimeError("VmRSS is zero; cannot verify memory budget")
                return value
    raise RuntimeError("VmRSS unavailable; cannot verify memory budget")


def run(binary, profile, scenario):
    fixture = tempfile.TemporaryDirectory(prefix="dark-terminal-")
    for args in (("init", "-q"), ("config", "user.name", "Dark Test"),
                 ("config", "user.email", "dark-test@example.invalid")):
        subprocess.run(("git", *args), cwd=fixture.name, check=True, stdout=subprocess.DEVNULL)
    with open(os.path.join(fixture.name, "sample.txt"), "w", encoding="utf-8") as f:
        f.write("before\n")
    subprocess.run(("git", "add", "sample.txt"), cwd=fixture.name, check=True)
    subprocess.run(("git", "commit", "-qm", "fixture"), cwd=fixture.name, check=True)
    with open(os.path.join(fixture.name, "sample.txt"), "w", encoding="utf-8") as f:
        f.write("after\n")
    with open(os.path.join(fixture.name, "mouse.txt"), "w", encoding="utf-8") as f:
        f.write("mouse_selection_unique\n")
    master, slave = pty.openpty()
    process = None
    transcript = bytearray()
    peak = 0
    idle_samples = []
    budget_error = None
    try:
        resize(slave, 24, 80)
        before = termios.tcgetattr(slave)

        def child_setup():
            resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
            os.setsid()
            fcntl.ioctl(slave, termios.TIOCSCTTY, 0)

        args = [binary]
        if scenario == "panic":
            args.append("--self-test-panic")
        elif scenario == "ctrlc":
            args.append("--debug")
        environment = os.environ.copy()
        environment["TERM"] = "xterm-256color"
        process = subprocess.Popen(args, stdin=slave, stdout=slave, stderr=slave,
                                   preexec_fn=child_setup, env=environment,
                                   close_fds=True, cwd=fixture.name)

        def sample():
            nonlocal peak
            if process.poll() is None:
                try:
                    value = rss(process.pid)
                except (FileNotFoundError, ProcessLookupError, RuntimeError):
                    # A process can lose VmRSS just before waitpid observes exit.
                    time.sleep(0.02)
                    if process.poll() is not None:
                        return None
                    raise
                peak = max(peak, value)
                return value
            return None

        def pump(wait=0.02):
            ready, _, _ = select.select([master], [], [], wait)
            if ready:
                try:
                    data = os.read(master, 65536)
                except OSError as error:
                    raise RuntimeError(f"PTY read failed: {error}") from error
                transcript.extend(data)
            if process.poll() is None:
                sample()

        def wait_for(expected, start):
            deadline = time.monotonic() + TIMEOUT
            while expected not in transcript[start:]:
                if process.poll() is not None:
                    raise RuntimeError(f"exited before {expected!r}; output={bytes(transcript)!r}")
                if time.monotonic() >= deadline:
                    raise RuntimeError(f"timed out waiting for {expected!r}; output={bytes(transcript)!r}")
                pump()

        def wait_body(expected, start):
            deadline = time.monotonic() + TIMEOUT
            marker = b"\x1b[13;1H"
            while True:
                chunk = bytes(transcript[start:])
                pos = chunk.rfind(marker)
                if pos >= 0:
                    body = chunk[pos + len(marker):].split(b"\x1b[14;1H", 1)[0]
                    if expected in body:
                        return
                if process.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError(f"body {expected!r} not seen; output={chunk!r}")
                pump()

        def wait_border(column, start):
            deadline = time.monotonic() + TIMEOUT
            while True:
                chunk = bytes(transcript[start:])
                pos = chunk.rfind(b"\x1b[13;1H")
                if pos >= 0:
                    row = chunk[pos + len(b"\x1b[13;1H"):].split(b"\x1b[14;1H", 1)[0]
                    row = re.sub(rb"\x1b\[[0-9;]*[A-Za-z]", b"", row).decode("utf-8", "replace")
                    if len(row) > column and row[column] == "\u2502":
                        return
                if process.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError(f"separator not at column {column}: {chunk!r}")
                pump()

        wait_for(b"Changes", 0)
        if scenario != "panic":
            wait_for(b"sample.txt", 0)
            wait_for(b"\x1b[24;1H", transcript.find(b"sample.txt"))
            os.write(master, b"qaxu")
            time.sleep(0.2)
            if process.poll() is not None:
                raise RuntimeError("bare q exited the application")
            # Click the untracked file, then the tracked file, using real SGR bytes.
            start = len(transcript)
            os.write(master, b"\x1b[<0;6;7M\x1b[<0;6;7m")
            wait_for(b"mouse_selection_unique", start)
            start = len(transcript)
            os.write(master, b"\x1b[<0;6;5M\x1b[<0;6;5m")
            wait_for(b"after", start)
            for key, label in ((b"2", b"Edit"), (b"3", b"Vibe"), (b"4", b"Git commit"),
                               (b"5", b"Models"), (b"6", b"View"), (b"7", b"Help")):
                start = len(transcript)
                os.write(master, key)
                wait_body(label, start)
            start = len(transcript)
            os.write(master, b"\x1b1")
            wait_for(b"Changes", start)
            # Test actual terminal bytes, not already-decoded synthetic KeyEvents.
            # Legacy Alt+[ is the ambiguous CSI prefix; CSI-u can encode it clearly.
            for sequence, column in ((b"\x1b=", 27), (b"\x1b-", 23),
                                     (b"\x1b=", 27), (b"\x1b0", 23),
                                     (b"\x1b=", 27), (b"\x1b[91;3u", 23)):
                start = len(transcript)
                os.write(master, sequence)
                wait_border(column, start)
            # SGR mouse coordinates are one-based; move the separator from
            # column 24 to column 28 and verify the rendered border moved.
            start = len(transcript)
            os.write(master, b"\x1b[<0;24;13M\x1b[<32;28;13M\x1b[<0;28;13m")
            wait_border(27, start)
            if scenario == "ctrlc" and b"Memory (rss):" not in transcript:
                raise RuntimeError("debug memory status missing")
            # A fresh frame after SIGWINCH must arrive before any subsequent key.
            start = len(transcript)
            resize(slave, 12, 60)
            os.kill(process.pid, signal.SIGWINCH)
            wait_for("Cửa sổ quá nhỏ".encode(), start)
            start = len(transcript)
            resize(slave, 24, 80)
            os.kill(process.pid, signal.SIGWINCH)
            wait_for(b"Changes", start)
            for _ in range(5):
                time.sleep(0.1)
                value = sample()
                if value is None:
                    raise RuntimeError("application exited during idle measurement")
                idle_samples.append(value)
            if profile == "release" and max(idle_samples) >= IDLE_LIMIT:
                budget_error = f"RAM budget exceeded: idle={idle_samples} peak_sampled={peak}"
            os.write(master, b"\x1b\x03" if scenario == "ctrlc" else b"\x1bq")

        deadline = time.monotonic() + TIMEOUT
        while process.poll() is None:
            if time.monotonic() >= deadline:
                raise RuntimeError("application did not exit")
            pump()
        # Drain the PTY while the slave remains open in this process.
        while select.select([master], [], [], 0)[0]:
            pump(0)
        if profile == "release" and peak >= PEAK_LIMIT:
            budget_error = f"RAM peak budget exceeded: peak_sampled={peak}"
        code = process.wait()
        if (scenario == "panic") == (code == 0):
            raise RuntimeError(f"unexpected exit code {code}")
        after = termios.tcgetattr(slave)
        if before != after:
            raise RuntimeError(f"terminal attributes not restored: {before!r} -> {after!r}")
        if b"\x1b[?1049h" not in transcript or b"\x1b[?1049l" not in transcript:
            raise RuntimeError("alternate screen entry/exit missing")
        if b"\x1b[?25l" not in transcript or b"\x1b[?25h" not in transcript:
            raise RuntimeError("cursor hide/show missing")
        if b"\x1b[?1000h" not in transcript or b"\x1b[?1000l" not in transcript:
            raise RuntimeError("mouse capture entry/exit missing")
        if budget_error:
            raise RuntimeError(budget_error)
        print(f"{profile} {scenario} PASS idle_max_5_samples_rss_bytes="
              f"{max(idle_samples) if idle_samples else 'n/a'} "
              f"peak_sampled_rss_bytes={peak} exit={code}", flush=True)
    finally:
        if process is not None and process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)
        fixture.cleanup()


def main():
    if not sys.platform.startswith("linux") or len(sys.argv) != 2:
        raise SystemExit("Linux and target directory required; cannot verify")
    failures = []
    for profile in ("debug", "release"):
        binary = os.path.abspath(os.path.join(sys.argv[1], profile, "dark"))
        if not os.path.isfile(binary):
            raise SystemExit(f"missing binary: {binary}")
        for scenario in ("q", "ctrlc", "panic"):
            try:
                run(binary, profile, scenario)
            except (OSError, RuntimeError) as error:
                failures.append(f"{profile} {scenario} FAIL: {error}")
                print(failures[-1], flush=True)
    if failures:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
