#!/usr/bin/env python3
"""PTY smoke test for mouse selection.

The pasture only takes mouse input from a real terminal, so this drives the
release binary in a pty against a stub `herdr`, decodes the frames it paints,
clicks a sheep, and checks what the pane says afterwards. Nothing here mocks
the plugin: the clicks are the same SGR reports a terminal sends.

Run: python3 scripts/smoke-click.py [--binary target/release/herdr-sheep]
"""

from __future__ import annotations

import argparse
import fcntl
import os
import pty
import re
import struct
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path

COLS, ROWS = 100, 38
# Two agents, one per zone, so the top zone holds exactly one known sheep.
SNAPSHOT = """{
  "id": "cli:api:snapshot",
  "result": {
    "type": "snapshot",
    "snapshot": {
      "focused_pane_id": "w9:p9",
      "workspaces": [{ "workspace_id": "w1", "label": "gate", "number": 1 },
                     { "workspace_id": "w2", "label": "meadow", "number": 2 }],
      "agents": [
        { "pane_id": "w1:p1", "workspace_id": "w1", "agent_status": "blocked",
          "agent": "omp", "name": "deploy", "tokens": { "provider": "claude" } },
        { "pane_id": "w2:p1", "workspace_id": "w2", "agent_status": "idle",
          "agent": "omp", "name": "scout", "tokens": { "provider": "codex" } }
      ]
    }
  }
}
"""

MOVE = re.compile(r"\x1b\[(\d+);(\d+)H")
ESCAPE = re.compile(r"\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*\x07")


def stub_herdr(directory: Path) -> tuple[Path, Path]:
    """A `herdr` that answers `api snapshot` and logs every call."""
    snapshot = directory / "snapshot.json"
    snapshot.write_text(SNAPSHOT)
    log = directory / "calls.log"
    log.touch()
    binary = directory / "herdr"
    binary.write_text(
        "#!/usr/bin/env bash\n"
        f'printf "%s\\n" "$*" >> {log}\n'
        f'if [ "$1 $2" = "api snapshot" ]; then cat {snapshot}; fi\n'
        "exit 0\n"
    )
    binary.chmod(0o755)
    return binary, log


class Pane:
    """A pty running the pasture, with the painted grid decoded from its output."""

    def __init__(self, binary: str, herdr: Path) -> None:
        primary, secondary = pty.openpty()
        fcntl.ioctl(secondary, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
        env = dict(os.environ, TERM="xterm-256color", HERDR_BIN_PATH=str(herdr))
        self.process = subprocess.Popen(
            [binary],
            stdin=secondary,
            stdout=secondary,
            stderr=subprocess.PIPE,
            env=env,
            close_fds=True,
        )
        os.close(secondary)
        self.fd = primary
        os.set_blocking(self.fd, False)
        self.grid = [[" "] * COLS for _ in range(ROWS)]
        self.row, self.col = 0, 0

    def pump(self, seconds: float = 0.4) -> None:
        """Read for `seconds` and apply everything painted onto the grid."""
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            try:
                chunk = os.read(self.fd, 65536)
            except BlockingIOError:
                time.sleep(0.02)
                continue
            if not chunk:
                break
            self.apply(chunk.decode("utf-8", "replace"))

    def apply(self, text: str) -> None:
        index = 0
        while index < len(text):
            if text[index] == "\x1b":
                match = MOVE.match(text, index)
                if match:
                    self.row = int(match.group(1)) - 1
                    self.col = int(match.group(2)) - 1
                    index = match.end()
                    continue
                other = ESCAPE.match(text, index)
                if other:
                    # Clears wipe the grid; every other escape is styling.
                    if other.group(0).endswith("J"):
                        self.grid = [[" "] * COLS for _ in range(ROWS)]
                    index = other.end()
                    continue
                index += 1
                continue
            if text[index] in "\r\n":
                index += 1
                continue
            if 0 <= self.row < ROWS and 0 <= self.col < COLS:
                self.grid[self.row][self.col] = text[index]
                self.col += 1
            index += 1

    def line(self, row: int) -> str:
        return "".join(self.grid[row]).rstrip()

    def screen(self) -> str:
        return "\n".join(self.line(row) for row in range(ROWS))

    def find(self, glyph: str, first_row: int = 0) -> tuple[int, int]:
        for row in range(first_row, ROWS):
            for col in range(COLS):
                if self.grid[row][col] == glyph:
                    return row, col
        raise AssertionError(f"no {glyph!r} below row {first_row}:\n{self.screen()}")

    def click(self, row: int, col: int) -> None:
        """One left click, as SGR mouse reports (`?1006` press then release)."""
        press = f"\x1b[<0;{col + 1};{row + 1}M"
        release = f"\x1b[<0;{col + 1};{row + 1}m"
        os.write(self.fd, (press + release).encode())

    def send(self, keys: str) -> None:
        os.write(self.fd, keys.encode())

    def close(self) -> int:
        self.send("q")
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            raise
        os.close(self.fd)
        return self.process.returncode


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default="target/release/herdr-sheep")
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    if not binary.exists():
        print(f"smoke-click: {binary} is not built; run `just install`", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as directory:
        herdr, log = stub_herdr(Path(directory))
        pane = Pane(str(binary), herdr)
        try:
            pane.pump(1.5)
            screen = pane.screen()
            assert "GATE" in screen, f"the flock never arrived:\n{screen}"
            assert "press j or k" in screen, f"something is already selected:\n{screen}"

            # The horn of the topmost sheep, which is the blocked one in GATE.
            # Row 0 is the header, whose mini sprite carries a horn of its own.
            row, col = pane.find("@", first_row=1)
            pane.click(row, col)
            pane.pump(1.0)
            detail = pane.line(ROWS - 2)
            assert "w1:p1" in detail, f"clicking the sheep did not select it: {detail!r}"
            assert "deploy" in pane.screen()

            # Clicking the same sheep twice focuses its pane.
            pane.click(row, col)
            pane.click(row, col)
            deadline = time.monotonic() + 3.0
            calls = ""
            while time.monotonic() < deadline:
                pane.pump(0.2)
                calls = log.read_text()
                if "agent focus w1:p1" in calls:
                    break
            assert "agent focus w1:p1" in calls, f"a double click did not focus:\n{calls}"

            code = pane.close()
            assert code == 0, f"the pasture exited with {code}"
        finally:
            if pane.process.poll() is None:
                pane.process.kill()

    print("smoke-click: click selects, double click focuses, q quits")
    return 0


if __name__ == "__main__":
    sys.exit(main())
