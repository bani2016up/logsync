use crate::domain::compare::Compare;
use crate::domain::{AutoTimestampSelector, SelectedTimestamp, TimestampSelector};
use chrono::{DateTime, Utc};
use std::fs;

pub struct LogEntry {
    pub(crate) timestamp: DateTime<Utc>,
    message: String,
}

pub(crate) struct LogFile<S: TimestampSelector = AutoTimestampSelector> {
    path: String,
    pub(crate) entities: Vec<LogEntry>,
    _selector: S,
}

impl LogEntry {
    pub fn get_log_message(&self) -> &str {
        &self.message
    }
}

impl LogFile<AutoTimestampSelector> {
    pub fn from_file(path: String) -> Self {
        Self::from_file_with_selector(path, AutoTimestampSelector)
    }

    #[cfg(test)]
    fn from_source(path: String, source: &str) -> Self {
        Self::from_source_with_selector(path, source, AutoTimestampSelector)
    }
}

impl<S: TimestampSelector> LogFile<S> {
    pub fn from_file_with_selector(path: String, selector: S) -> Self {
        let source = fs::read_to_string(&path).expect("Should have been able to read the file");
        Self::from_source_with_selector(path, &source, selector)
    }

    fn from_source_with_selector(path: String, source: &str, mut selector: S) -> Self {
        // Strip terminal commands before parsing timestamps or displaying messages.
        let mut source = strip_ansi_escapes::strip_str(&source.replace('\t', "    "));
        source.retain(|c| !c.is_control() || c == '\n');
        let mut entities: Vec<LogEntry> = Vec::new();

        for line in source.lines() {
            match selector.select(line) {
                Some(SelectedTimestamp { timestamp, message }) => {
                    entities.push(LogEntry {
                        timestamp,
                        message: message.to_owned(),
                    });
                }

                None => {
                    if let Some(last) = entities.last_mut() {
                        last.message.push('\n');
                        last.message.push_str(line);
                    }
                }
            }
        }

        LogFile {
            path,
            entities,
            _selector: selector,
        }
    }
}

impl Compare<DateTime<Utc>> for LogEntry {
    fn key(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::compare_logfiles::compare_logfiles;

    #[test]
    fn compares_different_formats_and_keeps_subsecond_entries_distinct() {
        let left = LogFile::from_source(
            String::new(),
            "2026-09-12 10:00:00.100 left\n2026-09-12 10:00:00.200 extra",
        );
        let right = LogFile::from_source(
            String::new(),
            "[2026-09-12T12:00:00.100+02:00] right\n2026/09/12 10:00:01 later",
        );
        let result = compare_logfiles(&left, &right);
        assert_eq!(result.left_container(), ["left", "extra", "\n"]);
        assert_eq!(result.right_container(), ["right", "\n", "later"]);
        assert_eq!(
            result.timestamps(),
            [
                "2026-09-12 10:00:00.100 UTC",
                "2026-09-12 10:00:00.200 UTC",
                "2026-09-12 10:00:01 UTC",
            ]
        );
    }

    #[test]
    fn accepts_a_custom_selector_independently_for_each_log() {
        struct CustomSelector;
        impl TimestampSelector for CustomSelector {
            fn select<'a>(&mut self, line: &'a str) -> Option<SelectedTimestamp<'a>> {
                let (timestamp, message) = line.strip_prefix("time=")?.split_once('|')?;
                Some(SelectedTimestamp {
                    timestamp: DateTime::parse_from_rfc3339(timestamp)
                        .ok()?
                        .with_timezone(&Utc),
                    message,
                })
            }
        }
        let left = LogFile::from_source_with_selector(
            String::new(),
            "time=2026-09-12T10:00:00Z|custom",
            CustomSelector,
        );
        let right = LogFile::from_source(String::new(), "2026-09-12 10:00:00 automatic");
        let result = compare_logfiles(&left, &right);
        assert_eq!(result.length(), 1);
        assert_eq!(result.left_container(), ["custom"]);
        assert_eq!(result.right_container(), ["automatic"]);
    }

    #[test]
    fn colored_logs_parse_and_compare_like_plain_text() {
        let plain = "2026-09-12 00:00:01 ERROR failed\n    RuntimeError: example\n2026-09-12 00:00:02 INFO recovered";
        let colored = "\x1b[32m2026-09-12 00:00:01\x1b[0m \x1b[31mERROR failed\x1b[0m\r\n\t\x1b[1;35mRuntimeError: example\x1b[0m\r\n\x1b[38;2;255;0;0m2026-09-12 00:00:02 INFO recovered\x1b[0m";
        let left = LogFile::from_source(String::new(), plain);
        let right = LogFile::from_source(String::new(), colored);

        assert_eq!(right.entities.len(), 2);
        assert_eq!(left.entities[0].timestamp, right.entities[0].timestamp);
        let result = compare_logfiles(&left, &right);
        assert_eq!(result.length(), 2);
        assert_eq!(result.left_container(), result.right_container());
    }

    #[test]
    fn removes_terminal_commands_and_preserves_link_text() {
        let source = "2026-09-12 00:00:01 INFO \x1b[2J\x1b[10;20H\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\ \u{00e9}\x07\x08";
        let log = LogFile::from_source(String::new(), source);

        assert_eq!(log.entities.len(), 1);
        assert_eq!(log.entities[0].get_log_message(), "INFO link \u{00e9}");
    }

    fn logfile(seconds: &[u32], label: &str) -> LogFile {
        LogFile {
            path: String::new(),
            _selector: AutoTimestampSelector,
            entities: seconds
                .iter()
                .map(|second| LogEntry {
                    timestamp: DateTime::parse_from_rfc3339(&format!(
                        "2026-09-12T00:00:{second:02}Z"
                    ))
                    .unwrap()
                    .with_timezone(&Utc),
                    message: format!("{label}{second}"),
                })
                .collect(),
        }
    }

    #[test]
    fn aligns_matching_and_unmatched_entries() {
        let left = logfile(&[1, 3, 4], "L");
        let right = logfile(&[2, 3, 5], "R");
        let result = compare_logfiles(&left, &right);

        assert_eq!(result.left_container(), ["L1", "\n", "L3", "L4", "\n"]);
        assert_eq!(result.right_container(), ["\n", "R2", "R3", "\n", "R5"]);
        assert_eq!(result.length(), 5);
        assert_eq!(left.entities.len(), 3);
        assert_eq!(right.entities.len(), 3);
    }

    #[test]
    fn handles_empty_logs_and_remaining_entries_on_either_side() {
        let empty = logfile(&[], "");
        let populated = logfile(&[1, 2], "L");
        let result = compare_logfiles(&empty, &empty);
        assert!(result.left_container().is_empty());
        assert!(result.right_container().is_empty());
        assert_eq!(result.length(), 0);

        let result = compare_logfiles(&populated, &empty);
        assert_eq!(result.left_container(), ["L1", "L2"]);
        assert_eq!(result.right_container(), ["\n", "\n"]);
        assert_eq!(result.length(), 2);

        let result = compare_logfiles(&empty, &populated);
        assert_eq!(result.left_container(), ["\n", "\n"]);
        assert_eq!(result.right_container(), ["L1", "L2"]);
        assert_eq!(result.length(), 2);
    }

    #[test]
    fn pairs_duplicate_timestamps_in_order() {
        let result = compare_logfiles(&logfile(&[1, 1], "L"), &logfile(&[1], "R"));
        assert_eq!(result.left_container(), ["L1", "L1"]);
        assert_eq!(result.right_container(), ["R1", "\n"]);
        assert_eq!(result.length(), 2);
    }
}
