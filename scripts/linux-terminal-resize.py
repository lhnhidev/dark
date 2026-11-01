#!/usr/bin/env python3
"""Resize the live terminal launched by scripts/linux-interactive.sh."""
import argparse
import fcntl
import os
from pathlib import Path
import signal
import struct
import termios

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("rows", type=int)
parser.add_argument("columns", type=int)
args = parser.parse_args()
if not (0 < args.rows <= 65535 and 0 < args.columns <= 65535):
    parser.error("rows and columns must be between 1 and 65535")
base = Path(__file__).resolve().parent.parent / ".dark/verification/manual"
pid = int((base / "pid.txt").read_text())
tty = (base / "tty.txt").read_text().strip()
if os.readlink(f"/proc/{pid}/fd/0") != tty:
    raise SystemExit("recorded terminal does not match the running process")
with open(tty, "rb", buffering=0) as stream:
    fcntl.ioctl(stream, termios.TIOCSWINSZ, struct.pack("HHHH", args.rows, args.columns, 0, 0))
os.kill(pid, signal.SIGWINCH)
print(f"resized pid={pid} tty={tty} to {args.columns} columns x {args.rows} rows; no key sent")
