# logsync

A small Rust terminal app for viewing multiple log files side by side, aligned by
timestamp. Built with [tui-rs](https://github.com/fdehau/tui-rs) and
[Crossterm](https://github.com/crossterm-rs/crossterm), with a deliberately simple
codebase for learning.

Example output for the versioned fixtures in `assets/`:

```text
┌Timestamp──────────────┐┌Log 1────────────────────────────────┐┌Log 2───────────────────────────────┐┌Log 3───────────────────────────────┐
│2026-09-12 10:00:00 UTC││INFO gateway started                 ││INFO worker ready                    ││                                    │
│2026-09-12 10:00:01 UTC││DEBUG cache miss for user-42         ││                                    ││DEBUG scheduled task                │
│2026-09-12 10:00:02 UTC││                                     ││WARNING queue depth high             ││                                    │
│2026-09-12 10:00:03 UTC││WARN retrying upstream request       ││FATAL worker stopped                 ││INFO fallback response              │
│2026-09-12 10:00:04 UTC││                                     ││                                    ││PANIC storage unavailable           │
│                       ││                                     ││                                    ││    at storage.rs:42                │
│2026-09-12 10:00:05 UTC││ERROR request failed                 ││                                    ││                                    │
│                       ││    RuntimeError: connection refused ││                                    ││                                    │
└───────────────────────┘└─────────────────────────────────────┘└────────────────────────────────────┘└────────────────────────────────────┘
```

Log levels are colorized in the terminal; the integration snapshot verifies this
output and its colors.

## Run

Install a current stable [Rust toolchain](https://rustup.rs/) supporting edition
2024, then run in an interactive terminal:

```sh
git clone https://github.com/bani2016up/logsync.git
cd logsync
cargo run --release -- /path/to/first.log /path/to/second.log /path/to/third.log
```

Or install the binary from your local checkout:

```sh
cargo install --path . --locked
logsync /path/to/first.log /path/to/second.log /path/to/third.log
```

Pass two or more file paths. Each file gets a column labeled `Log 1`, `Log 2`,
`Log 3`, and so on, in argument order. Two-file comparisons still work.
There are currently no command-line flags. The example above shows a three-log
comparison.

## Controls

| Input | Action |
| --- | --- |
| Up / Down | Scroll all logs by one aligned entry |
| Left / Right | Scroll all message columns horizontally |
| Vertical mouse wheel or touchpad scroll | Scroll vertically |
| Horizontal mouse wheel or touchpad scroll | Scroll horizontally |
| Shift + vertical wheel | Horizontal scrolling fallback |
| `q` / Esc | Quit |

The timestamp column stays fixed during horizontal scrolling. Mouse and
touchpad support depends on the terminal forwarding the appropriate events.

## Log Format

All files must be UTF-8 and already sorted by timestamp (after UTC normalization).
The default selector detects a supported timestamp at the start of each line,
optionally surrounded by square brackets. For example:

```text
2026-09-12 10:00:00 INFO starting
[2026-09-12T10:00:01.123Z] ERROR request failed
    RuntimeError: connection refused
2026-09-12 10:00:02 INFO retrying
```

| Format | Example |
| --- | --- |
| Year-first date and time | `2026-09-12 10:00:00` |
| Slash or dot date separators | `2026/09/12 10:00:00`, `2026.09.12 10:00:00` |
| ISO 8601 / RFC 3339 | `2026-09-12T12:00:00.123456789+02:00` |
| Comma fractions | `2026-09-12 10:00:00,123` |
| Explicit UTC or numeric offset | `2026-09-12 10:00:00 UTC`, `2026-09-12 12:00:00 +0200` |
| RFC 2822 | `Sat, 12 Sep 2026 10:00:00 +0000` |
| Apache timestamp | `[12/Sep/2026:12:00:00 +0200]` |
| Unix seconds / milliseconds / microseconds / nanoseconds | `1700000000`, `1700000000123`, `1700000000123456`, `1700000000123456789` |

- Detection runs per entry, so files can use different formats or mix supported
  formats. Timestamp and message must be separated by whitespace; a timestamp
  can also occupy the whole line. Leading whitespace is accepted.
- Explicit time zones are normalized to UTC for comparison and display.
  **Timestamps without a time zone are assumed to be UTC**, not machine-local time.
- Fractional seconds are preserved up to nanosecond precision. Entries at
  `.100` and `.200` are distinct; equivalent instants with different offsets align.
- Epoch units are inferred from 10, 13, 16, or 19 digits (an optional minus sign
  is not counted). Other digit lengths are not auto-detected.
- Ambiguous dates such as `01/02/2026`, time-only values, missing-year syslog
  timestamps, named zones such as `EST`, arbitrary prefixes, and other formats
  need a custom selector. This is detection of supported formats, not a parser
  for every possible date notation.
- Lines without a recognized timestamp attach to the previous entry, supporting
  multiline messages and tracebacks. Lines before the first entry are ignored.
- Each row uses the earliest pending timestamp across all files. Entries with
  that timestamp are aligned, and files without a matching entry get a blank.
  Duplicate timestamps are paired in encounter order, one entry per file per row.
- Multiline rows are padded to the tallest entry across all files so the next
  entries remain aligned. Empty files keep their own blank columns.
- ANSI escape sequences are stripped before parsing. Tabs become four spaces.
  The viewer colors `ERROR`, `FATAL`, and `PANIC` red; `WARN` and `WARNING`
  yellow; `INFO` green; `DEBUG` blue; and `TRACE` gray. Original files are not
  modified.

This is a timestamp-aligned viewer, not a message diff: different messages at
the same timestamp are shown together without difference highlighting.

## Custom Selectors

`LogFile<S>` is generic over `TimestampSelector`, defaulting to
`AutoTimestampSelector`. The CLI uses the default automatically:

```rust
let log = LogFile::from_file(path);
```

To support another layout, implement the trait inside the app and supply an
instance to `LogFile::from_file_with_selector(path, selector)`. A selector receives
a sanitized line and returns a UTC timestamp and a borrowed message body, or
`None` for a continuation line. It can maintain state through `&mut self`.

```rust
use chrono::{DateTime, Utc};
use crate::domain::{LogFile, SelectedTimestamp, TimestampSelector};

struct TaggedTimestampSelector;

impl TimestampSelector for TaggedTimestampSelector {
    fn select<'a>(&mut self, line: &'a str) -> Option<SelectedTimestamp<'a>> {
        // Example: time=2026-09-12T10:00:00Z|INFO starting
        let (timestamp, message) = line.strip_prefix("time=")?.split_once('|')?;
        Some(SelectedTimestamp {
            timestamp: DateTime::parse_from_rfc3339(timestamp).ok()?.with_timezone(&Utc),
            message,
        })
    }
}

// In a function:
// let log = LogFile::from_file_with_selector(path, TaggedTimestampSelector);
```

Pass a vector of log references to compare any number of files:

```rust
let result = compare_logfiles(vec![&first, &second, &third]);
```

Comparison accepts inputs implementing `AsRef<[LogEntry]>`. For logs with
different selector types, pass their entry slices without copying:

```rust
let result = compare_logfiles(vec![automatic.as_ref(), custom.as_ref()]);
```

`LogEntry` stores the parsed timestamp separately from its message body;
`CompareResult::new(containers, timestamps)` validates equal lengths for every
message column and the timestamp column. Its getters are read-only. Zero input
columns are allowed only with an empty timestamp column. The renderer does not
parse timestamp strings.

## Limitations

- All files and their aligned messages are held in memory; there is no live
  following or streaming mode.
- Unreadable files or invalid UTF-8 currently cause an error panic.
- Long lines are clipped, not wrapped; use horizontal scrolling to read them.
- Log columns share the available terminal width. With many files, use a wider
  terminal or compare fewer files if the panes become too narrow to read.
- Vertical scrolling moves by entry. A multiline entry taller than the viewport
  cannot currently be scrolled internally to reveal its lower lines.
- There is no search, filtering, clock-offset correction, or export yet.

## Code Layout

```text
src/
  main.rs                      Load files and launch the viewer
  application/
    compare_logfiles.rs         Align all logs by timestamp
  domain/
    compare.rs                 Comparison-key trait
    compare_result.rs          Validated, read-only aligned columns
    logfile.rs                 Log loading and parsing
    timestamp_selector.rs      Selector trait and automatic format detection
  tui/
    mod.rs                     Terminal setup, events, and cleanup
    render.rs                  Timestamp pane and dynamic log columns
    state.rs                   Vertical and horizontal scroll offsets
```

## Development

```sh
cargo fmt --check
cargo check --locked
cargo test --locked
```

Tests cover parsing colored logs, alignment, result invariants, scrolling, and
rendering through tui-rs's in-memory test backend. The integration snapshot
loads the three versioned `assets/integration-log-*.log` fixtures and verifies
the complete rendered table and its colors. Local `*.log` files outside
`assets/`, `.env` files, and build output are ignored by Git. Do not submit
private logs or secrets.

## License

Licensed under the [MIT License](LICENSE).
