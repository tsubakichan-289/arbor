#!/usr/bin/env python3
"""Linux PTY integration checks; run after cargo build --bins --examples.

Checks actual raw-mode flags, key handling, SIGWINCH, alternate-screen cleanup,
returned-error cleanup, and panic cleanup. No graphical terminal is required.
"""
import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time


ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target/debug/arbor"
PROBE = ROOT / "target/debug/examples/terminal_probe"


def run_case(command, exit_key=None, navigate=False, respond_cursor=True):
    master, slave = pty.openpty()
    before = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 100, 0, 0))

    def controlling_terminal():
        os.setsid()
        fcntl.ioctl(slave, termios.TIOCSCTTY, 0)

    process = subprocess.Popen(
        [str(part) for part in command], stdin=slave, stdout=slave, stderr=slave,
        env={**os.environ, "TERM": "xterm-256color"},
        preexec_fn=controlling_terminal,
    )
    output = bytearray()
    cursor_replies = 0

    def drain(duration=0.15):
        nonlocal cursor_replies
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            ready, _, _ = select.select([master], [], [], max(0, deadline - time.monotonic()))
            if ready:
                output.extend(os.read(master, 65536))
                # A bare PTY has no terminal emulator to answer the DSR query.
                # Ratatui asks for the initial cursor position during setup.
                requests = output.count(b"\x1b[6n")
                while respond_cursor and cursor_replies < requests:
                    os.write(master, b"\x1b[1;1R")
                    cursor_replies += 1

    try:
        if exit_key is not None:
            deadline = time.monotonic() + 10
            while b"ARBOR" not in output and process.poll() is None and time.monotonic() < deadline:
                drain()
            assert b"ARBOR" in output, output.decode(errors="replace")
            assert b"projects/" in output
            assert b"Cargo.toml" in output
            raw = termios.tcgetattr(slave)
            assert not raw[3] & (termios.ICANON | termios.ECHO), "raw mode was not enabled"
            if navigate:
                # Enter on Cargo.toml (the preview's middle item), then move
                # down, up twice to src/, enter it, and return twice.
                for key in [b"\x1b[C", b"\x1b[B", b"\x1b[A", b"\x1b[A", b"\x1b[C", b"\x1b[D", b"\x1b[D"]:
                    previous_length = len(output)
                    os.write(master, key)
                    drain()
                    assert process.poll() is None, "navigation unexpectedly exited"
                    assert len(output) > previous_length, "key did not redraw the screen"
                for rows, columns in [(3, 5), (1, 1), (40, 120)]:
                    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0))
                    os.kill(process.pid, signal.SIGWINCH)
                    drain()
                    assert process.poll() is None, "resize unexpectedly exited"
            os.write(master, exit_key)
        deadline = time.monotonic() + 10
        while process.poll() is None and time.monotonic() < deadline:
            drain()
        code = process.wait(timeout=1)
        drain(0.05)
        assert (code == 0) == (exit_key is not None), (code, output.decode(errors="replace"))
        assert termios.tcgetattr(slave) == before, "terminal attributes were not restored"
        assert b"\x1b[?1049h" in output, "alternate screen was not entered"
        assert b"\x1b[?1049l" in output, "alternate screen was not restored"
        assert b"\x1b[?25h" in output, "cursor was not restored"
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)


def main():
    with tempfile.TemporaryDirectory(prefix="arbor-pty-") as directory:
        root = Path(directory)
        (root / "projects/src").mkdir(parents=True)
        (root / "projects/Cargo.toml").write_text("# fixture\n")
        (root / "projects/README.md").write_text("fixture\n")
        (root / "readme").write_text("fixture\n")
        for label, key, navigate in [("arrows + resize + q", b"q", True), ("Esc", b"\x1b", False), ("Ctrl-C", b"\x03", False)]:
            run_case([BIN, root], key, navigate)
            print(f"PASS: {label}; raw mode, alternate screen and cursor restored")
        run_case([BIN, root], respond_cursor=False)
        print("PASS: initialization failure; raw mode, alternate screen and cursor restored")
    for mode in ["error", "panic"]:
        run_case([PROBE, mode])
        print(f"PASS: {mode}; raw mode, alternate screen and cursor restored")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, subprocess.TimeoutExpired) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
