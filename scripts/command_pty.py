#!/usr/bin/env python3
"""End-to-end command mode / foreground child / shell / cwd-file checks on Linux."""
import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target/debug/arbor"


class Session:
    def __init__(self, root, shell="/bin/sh"):
        self.master, self.slave = pty.openpty()
        self.before = termios.tcgetattr(self.slave)
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 120, 0, 0))
        self.cwd_file = root / "cwd-output"
        self.cwd_file.touch(mode=0o600)
        self.cwd_file.chmod(0o600)
        self.output = bytearray()
        self.replies = 0

        def controlling_terminal():
            os.setsid()
            fcntl.ioctl(self.slave, termios.TIOCSCTTY, 0)

        self.process = subprocess.Popen(
            [str(BIN), "--cwd-file", str(self.cwd_file), str(root)],
            stdin=self.slave, stdout=self.slave, stderr=self.slave,
            env={**os.environ, "TERM": "xterm-256color", "SHELL": str(shell), "HOME": str(root), "ENV": "", "PS1": "ARBOR_SHELL> "},
            preexec_fn=controlling_terminal,
        )
        self.expect(b"ARBOR", 0)
        self.assert_raw(True)

    def drain(self, duration=0.1):
        end = time.monotonic() + duration
        while time.monotonic() < end:
            ready, _, _ = select.select([self.master], [], [], max(0, end - time.monotonic()))
            if ready:
                self.output.extend(os.read(self.master, 65536))
                queries = self.output.count(b"\x1b[6n")
                while self.replies < queries:
                    os.write(self.master, b"\x1b[1;1R")
                    self.replies += 1

    def send(self, data):
        mark = len(self.output)
        os.write(self.master, data.encode() if isinstance(data, str) else data)
        self.drain()
        return mark

    def expect(self, token, mark, timeout=8):
        end = time.monotonic() + timeout
        while token not in self.output[mark:] and time.monotonic() < end and self.process.poll() is None:
            self.drain()
        assert token in self.output[mark:], (token, self.output[mark:].decode(errors="replace"))

    def assert_raw(self, enabled):
        flags = termios.tcgetattr(self.slave)[3]
        assert bool(flags & termios.ICANON) != enabled
        assert bool(flags & termios.ECHO) != enabled

    def completed(self, mark, expected=b"status 0"):
        self.expect(expected, mark)
        self.assert_raw(True)
        assert b"\x1b[?1049l" not in self.output[mark:]
        assert self.process.poll() is None
        # Results leave an empty command input ready for the next command.
        self.send(b"\x1b")

    def command(self, text, expected=b"status 0"):
        mark = self.send(":" + text + "\r")
        self.completed(mark, expected)

    def finish(self, expected_directory):
        self.send("q")
        assert self.process.wait(timeout=5) == 0
        self.drain()
        assert termios.tcgetattr(self.slave) == self.before
        assert self.cwd_file.read_bytes() == os.fsencode(expected_directory) + b"\0"

    def close(self):
        if self.process.poll() is None:
            # Also terminate any foreground child so a failed test leaves no jobs.
            try:
                group = os.tcgetpgrp(self.master)
                if group != self.process.pid:
                    os.killpg(group, signal.SIGKILL)
            except OSError:
                pass
            self.process.kill()
            self.process.wait()
        os.close(self.master)
        os.close(self.slave)


def main():
    with tempfile.TemporaryDirectory(prefix="arbor-command-") as tmp:
        root = Path(tmp)
        child = root / "a space"
        child.mkdir()
        (child / "inside").write_text("fixture")
        session = Session(root)
        try:
            # Build a command, navigate with Tab, and insert the new current path.
            session.send(":printf '%s' ")
            session.send(b"\t\x1b[C\t\x04")
            mark = session.send(" > selected-path\r")
            session.completed(mark)
            assert (child / "selected-path").read_bytes() == os.fsencode(child)
            # Selection survives reload even after a new file changes the list.
            for shortcut, output_name, expected in [(b"\x06", "inserted-name", b"inside"), (b"\x10", "inserted-path", os.fsencode(child / "inside"))]:
                session.send(":printf '%s' ")
                session.send(shortcut)
                mark = session.send(" > " + output_name + "\r")
                session.completed(mark)
                assert (child / output_name).read_bytes() == expected
            session.send(":pwd\r")
            session.send(b"\x1b")
            session.command("touch new.txt; mkdir new-dir")
            assert (child / "new.txt").exists() and (child / "new-dir").is_dir()
            session.command("exit 7", b"status 7")
            # Output is visible before completion, without leaving the TUI.
            mark = session.send(":printf RUNNING; sleep 30\r")
            session.expect(b"RUNNING", mark)
            session.assert_raw(True)
            assert os.tcgetpgrp(session.master) == session.process.pid
            fcntl.ioctl(session.slave, termios.TIOCSWINSZ, struct.pack("HHHH", 20, 90, 0, 0))
            os.kill(session.process.pid, signal.SIGWINCH)
            session.send(b"\x03")
            session.completed(mark, b"signal 2")
            # Interactive shell changes cwd internally, but Arbor retains its own cwd.
            mark = session.send("!")
            session.expect(b"ARBOR_SHELL>", mark)
            session.send("cd /; printf SHELL_OK\\n\n")
            # Interactive programs may change tty flags; restoration still applies.
            session.send("stty -echo\n")
            mark = session.send("exit\n")
            session.expect(b"\x1b[?1049h", mark)
            session.assert_raw(True)
            session.command("printf preserved > shell-cwd")
            assert (child / "shell-cwd").read_text() == "preserved"
            # Compound cd belongs to the shell; standalone cd changes Arbor.
            session.command("cd / && true")
            session.send(":cd ..\r")
            session.send(b"\x1b")
            session.send(":cd -\r")
            session.send(b"\x1b")
            session.command("printf stdout-result; printf stderr-result >&2")
            session.finish(child)
            print("PASS: command editing/focus/insertion, builtins, external status, reload, Ctrl-C, interactive shell, cwd-file and terminal restoration")
        finally:
            session.close()
    with tempfile.TemporaryDirectory(prefix="arbor-cargo-") as tmp:
        root = Path(tmp)
        (root / "src").mkdir()
        (root / "Cargo.toml").write_text('[package]\nname="arbor-command-fixture"\nversion="0.1.0"\nedition="2024"\n')
        (root / "src/lib.rs").write_text("#[test] fn smoke() { assert_eq!(2 + 2, 4); }\n")
        (root / "empty").mkdir()
        session = Session(root)
        try:
            session.command("cargo test --offline")
            session.send(":cd empty\r")
            session.send(b"\x1b")
            session.command("rmdir ../empty")
            session.finish(root)
            print("PASS: cargo test streams output; deleting the current directory returns to a valid ancestor")
        finally:
            session.close()
    with tempfile.TemporaryDirectory(prefix="arbor-spawn-") as tmp:
        root = Path(tmp)
        bad_shell = root / "bad-shell"
        bad_shell.write_bytes(b"#!/arbor-test-missing-interpreter\n")
        bad_shell.chmod(0o700)
        session = Session(root, bad_shell)
        try:
            # Ratatui can retain unchanged letters from the previous status row.
            session.command("true", b"such file or directory")
            session.finish(root)
            print("PASS: shell spawn failure returns to a working TUI")
        finally:
            session.close()


if __name__ == "__main__":
    main()
