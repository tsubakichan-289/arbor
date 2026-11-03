# Arbor

A terminal file browser and command environment written in Rust. Arbor displays directory hierarchies as columns connected by branches, keeping the current selection centered as you navigate.

Browsing is read-only. Commands you explicitly enter run with your user account's permissions.

```text
 documents/                       ╭── src/
                                  │
 projects/  ────── mycelium/  ─────┼── Cargo.toml
                                  │
 pictures/                        ╰── README.md
```

## Getting started

Arbor targets Linux and stable Rust. The documented development environment uses Rust/Cargo 1.95.0. Both input and output must be attached to an interactive TTY.

```bash
git clone https://github.com/tsubakichan-289/arbor.git
cd arbor
cargo run -- ~/projects
cargo run -- /etc
cargo run                     # Start in the current working directory
cargo run -- --help
```

A Nix development shell is included:

```bash
nix-shell
cargo run -- ~/projects
```

To install the command:

```bash
cargo install --path . --locked
arbor ~/projects
```

The shell expands `~`. A nonexistent path, a regular file, or a starting path that is itself a symbolic link is rejected. Intermediate path components follow normal OS resolution, and the starting directory is normalized to an absolute path.

The terminal interface uses Ratatui 0.30.2 and Crossterm 0.29, with `unicode-width` and `unicode-segmentation` for text layout. Resolved dependency versions are recorded in `Cargo.lock`.

## Navigation

| Key | Action |
| --- | --- |
| ↑ / ↓ | Select the previous or next item in the current column |
| ← | Return to the parent and reselect the directory you came from |
| → | Enter a directory; files and symbolic links are not entered |
| `:` | Enter command mode |
| `!` | Open an interactive shell in the current directory |
| `q`, Esc, Ctrl+C | Exit and restore the terminal |

The selection stays centered. Vertical navigation moves only the current column's list; ancestor columns and their horizontal connections retain their vertical positions. Horizontal navigation shifts the visible hierarchy.

Directories end in `/`, symbolic links in `@`, and hidden files are shown. Directories sort before other entries, using the original `OsString` names within each group. Invalid UTF-8 is replaced only for display; original paths are retained. Control characters in filenames are escaped.

An empty directory shows `(empty)`. A failed directory read shows `(error)`, with details in the status line.

## Command mode

Press `:` to open the input line. A result area directly above it shows standard output, standard error, and exit status. Output updates while a command runs, and Ctrl+C interrupts the command. On completion, Arbor returns to an empty input line without requiring an extra Enter press.

The result area occupies up to one third of the terminal height, capped at eight lines. It shows the tail of wrapped output, retains up to 64 KiB, and indicates truncation. ANSI colors and terminal control sequences are removed. Results remain visible until the next command.

| Key | Action in command mode |
| --- | --- |
| Text, Backspace, Delete | Insert text or remove the previous/current grapheme |
| ← / →, Home / End | Move the text cursor |
| Ctrl+F | Insert the selected entry's filename |
| Ctrl+P | Insert the selected entry's absolute path |
| Ctrl+D | Insert Arbor's current directory |
| Tab | Switch focus between the input line and file browser |
| Enter | Execute when the input line has focus |
| Esc, Ctrl+C | Cancel and return to normal navigation |

While the browser has focus, arrow keys select entries and navigate directories without losing the command text or cursor position. The path insertion shortcuts work with either focus. `q` and Enter do not accidentally exit or execute while browsing in command mode.

For example, type `:diff `, press Tab to select a file, and press Ctrl+P to insert its path. Select and insert the second path, return to the input with Tab, and press Enter. Arbor inserts separating spaces when needed.

Input editing operates on Unicode grapheme clusters, preserving Japanese text, combining characters, and emoji. Long input scrolls horizontally to keep the cursor visible.

### Path quoting

Inserted names and paths use POSIX single-quote escaping. Relative filenames beginning with `-` receive a `./` prefix. Spaces, newlines, and shell metacharacters remain part of a single argument.

Insertion is rejected when the cursor is inside quoting, escaping, expansion, a comment, a here-document, or another context where a separate argument cannot be guaranteed. Prefixes containing expansions or multiple lines are also conservatively rejected. Move the cursor to a normal argument position before inserting.

Non-UTF-8 paths are not converted lossily for command insertion. Supported quoting conventions are those of sh, bash, zsh, dash, ksh, and ash. Other shells can execute manually entered commands, but automatic path insertion is disabled.

### Built-ins and external processes

`cd` and `pwd` operate on Arbor's current directory. Examples include:

```sh
cd
cd ..
cd -
cd src
cd ~/projects
cd '/path/with spaces'
pwd
```

With no argument, `cd` goes to `HOME`. `cd -` returns to the directory preceding the latest navigation, including arrow-key navigation. Quoting and backslash escapes are supported, as are unquoted `~` and `~/...`. Variable substitution, command substitution, globbing, and brace expansion are not supported inside simple built-ins.

Compound commands such as `cd src && cargo test` are passed to the external shell. Their directory changes do not affect Arbor. Failed built-in navigation preserves the current directory, selection, and input.

External commands use `$SHELL -c`; interactive shells use `$SHELL -i`. If `SHELL` is unset or unavailable, Arbor falls back to `/bin/sh`.

Commands entered through `:` run without a controlling terminal and receive EOF on standard input. Standard output and standard error share a captured stream. Use `!` for editors, interactive programs, or commands that require terminal input. Exit the interactive shell with `exit`; its internal `cd` commands do not change Arbor's location.

Captured commands run in a separate session/process group, and Ctrl+C is forwarded to that group. For interactive shells, Arbor releases raw mode and the alternate screen, gives the child foreground terminal ownership, and restores terminal settings when it returns. Directory listings are refreshed on return. If an entry has disappeared, Arbor selects a nearby entry; if the current directory has become inaccessible, it falls back to an accessible ancestor.

## Change the launching shell's directory on exit

After installing the binary, source the appropriate wrapper from your shell configuration. Replace `/path/to/arbor` with your checkout location. Arbor does not edit your shell configuration automatically.

For Bash, add to `~/.bashrc`:

```bash
source /path/to/arbor/scripts/arbor.bash
```

For Zsh, add to `~/.zshrc`:

```zsh
source /path/to/arbor/scripts/arbor.zsh
```

Run `arbor ~/projects`. On successful exit, the launching shell changes to the final directory. Use `command arbor` to call the binary without the wrapper.

The wrapper creates a temporary file and passes it through `--cwd-file FILE`. Arbor writes an absolute path as raw bytes followed by NUL only on successful exit. The wrapper reads the path and invokes its own `cd`, without evaluating the path as code. It preserves arguments, exit status, and unusual filename bytes, and removes the temporary file on both success and failure.

`--cwd-file` requires an existing regular file accessible only to its owner, such as a mode-0600 file with one hard link. Arbor opens it once without following symbolic links and does not change its contents before successful exit.

## Layout and architecture

Arbor loads the ancestor path from the filesystem root at startup. Off-screen ancestors remain in the logical path. The current column's selection is centered in the browser area, excluding the header, command output, and status/input lines. Those reserved areas have the same height in normal and command modes, so switching modes does not move the selection.

Columns are 12–32 terminal cells wide, with six cells between them. Labels are shortened at grapheme boundaries. Connector segments combine directional bits before rendering, keeping their shape independent of drawing order. Only visible rows and clipped connector segments are drawn. Directory reads and sorting are synchronous.

| Location | Responsibility |
| --- | --- |
| `src/app/` | Navigation, application modes, focus, event handling, CLI validation |
| `src/command/` | Input editing, quoting, built-ins, captured processes, shells, cwd files |
| `src/fs/` | Directory sources, original names and paths, scanning, sorting |
| `src/layout/` | Pure coordinate calculations, columns, connectors, clipping, viewport |
| `src/ui/` | Ratatui widgets, rendering, terminal restoration, command output |
| `tests/` | Layout, navigation, filesystem, commands, rendering, shell integration |
| `scripts/arbor.bash`, `scripts/arbor.zsh` | Launching-shell directory integration |
| `scripts/pty_smoke.py`, `scripts/command_pty.py` | Linux pseudo-terminal checks |

`layout` does not access the filesystem, and `ui` renders coordinates supplied by the layout layer. `DirectorySource` can be replaced for tests. `Application` owns interaction modes and command input/output; `BrowserState` owns the current directory and selected path.

## Development checks

```bash
cargo fmt --all --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Tests cover selection centering, stable ancestors, connector composition, large-list clipping, Unicode, narrow terminals, resizing, navigation, filesystem errors, command execution, quoting, output limits, and shell wrappers. Permission tests require a user for whom access restrictions can be reproduced; they report when that is not possible. Zsh-specific checks report a skip if Zsh is unavailable.

To exercise real terminal restoration and command interaction using Python's standard library:

```bash
cargo build --bins --examples
python3 scripts/pty_smoke.py
python3 scripts/command_pty.py
```

Or use Nix with Python and Zsh:

```bash
nix-shell -p cargo rustc rustfmt clippy python3 zsh --run \
  'cargo test && cargo build --bins --examples && python3 scripts/pty_smoke.py && python3 scripts/command_pty.py'
```

The PTY scenarios cover normal exit, interruption, initialization failures, runtime errors, panic, interactive shell return, terminal setting changes, path insertion, and cwd-file output. The documented checks use Ratatui's test backend and Linux PTYs; manual visual testing in an external terminal emulator has not been recorded.

## Limitations

- Directory reads and sorting are synchronous. Large directories and network filesystems can delay input, and memory use depends on the full listing size.
- Listings are snapshots. There is no filesystem watcher; refreshes occur after external commands and interactive shells return.
- Built-in copy, move, delete, rename, search, mouse input, symlink traversal, asynchronous scanning, and configuration files are not implemented.
- Command history retains up to 100 entries in the current session, but recall keys, completion, and persistent history are not implemented.
- Captured output shows only its tail, without scrollback or color reproduction. A child program's buffering can delay visible output.
- There is no embedded PTY or custom job manager. Detached background jobs are not tracked. A top-level interactive shell stopped with Ctrl+Z is resumed automatically; job control inside that shell remains its responsibility.
- Emoji widths depend on the terminal and font. Very small terminals cannot display every UI element. SIGKILL and terminal disconnection cannot be handled by the normal restoration path.

## License

[MIT](LICENSE).

---

[日本語版 README](README.ja.md)
