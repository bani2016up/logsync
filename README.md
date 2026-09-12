# logsync

A small Rust terminal app for viewing two log files side by side, aligned by
timestamp. Built with [tui-rs](https://github.com/fdehau/tui-rs) and
[Crossterm](https://github.com/crossterm-rs/crossterm), with a deliberately simple
codebase for learning.

```text
Timestamp           | Left                  | Right
2026-09-12 10:00:00 | INFO starting         | INFO starting
2026-09-12 10:00:01 | WARNING retrying      |
2026-09-12 10:00:02 |                       | INFO connected
2026-09-12 10:00:03 | INFO ready            | ERROR failed
```

## Run

Install a current stable [Rust toolchain](https://rustup.rs/) supporting edition
2024, then run in an interactive terminal:

```sh
git clone https://github.com/bani2016up/logsync.git
cd logsync
cargo run --release -- /path/to/left.log /path/to/right.log
```

Or install the binary from your local checkout:

```sh
cargo install --path . --locked
logsync /path/to/left.log /path/to/right.log
```

Pass exactly two file paths. There are currently no command-line flags.

## Controls

| Input | Action |
| --- | --- |
| Up / Down | Scroll both logs by one aligned entry |
| Left / Right | Scroll both message columns horizontally |
| Vertical mouse wheel or touchpad scroll | Scroll vertically |
| Horizontal mouse wheel or touchpad scroll | Scroll horizontally |
| Shift + vertical wheel | Horizontal scrolling fallback |
| `q` / Esc | Quit |

The timestamp column stays fixed during horizontal scrolling. Mouse and
touchpad support depends on the terminal forwarding the appropriate events.

## Log Format

Both files must be UTF-8 and already sorted by timestamp. Each entry starts with
a timestamp in `YYYY-MM-DD HH:MM:SS` format:

```text
2026-09-12 10:00:00 INFO starting
2026-09-12 10:00:01 ERROR request failed
    RuntimeError: connection refused
2026-09-12 10:00:02 INFO retrying
```

- Lines without a recognized timestamp attach to the previous entry, supporting
  multiline messages and tracebacks. Lines before the first entry are ignored.
- Entries with equal timestamps are paired in encounter order. Unmatched entries
  leave a blank on the other side.
- Matching uses only the first 19 timestamp characters, with second precision.
  Fractional seconds and time zones are not part of the comparison key.
- ANSI escape sequences are stripped before parsing. Colored logs display as
  plain text; tabs become four spaces. Original files are not modified.

This is a timestamp-aligned viewer, not a message diff: different messages at
the same timestamp are shown together without difference highlighting.

## Limitations

- Both files and their aligned messages are held in memory; there is no live
  following or streaming mode.
- Unreadable files or invalid UTF-8 currently cause an error panic.
- Long lines are clipped, not wrapped; use horizontal scrolling to read them.
- Vertical scrolling moves by entry. A multiline entry taller than the viewport
  cannot currently be scrolled internally to reveal its lower lines.
- There is no search, filtering, clock-offset correction, or export yet.

## Code Layout

```text
src/
  main.rs                      Load files and launch the viewer
  application/
    compare_logfiles.rs         Align the two logs
  domain/
    compare.rs                 Comparison-key trait
    compare_result.rs          Validated, read-only aligned columns
    logfile.rs                 Log loading and parsing
  tui/
    mod.rs                     Terminal setup, events, and cleanup
    render.rs                  Three-column layout and drawing
    state.rs                   Vertical and horizontal scroll offsets
```

## Development

```sh
cargo fmt --check
cargo check --locked
cargo test --locked
```

Tests cover parsing colored logs, alignment, result invariants, scrolling, and
rendering through tui-rs's in-memory test backend. Local `*.log` files, `.env`
files, and build output are ignored by Git. Do not submit private logs or secrets.

## License

Licensed under the [MIT License](LICENSE).
