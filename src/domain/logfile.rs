use crate::domain::compare::Compare;
use crate::domain::{AutoTimestampSelector, SelectedTimestamp, TimestampSelector};
use chrono::{DateTime, Utc};
use std::fs;

pub struct LogEntry {
    pub(crate) timestamp: DateTime<Utc>,
    message: String,
}

pub struct LogFile<S: TimestampSelector = AutoTimestampSelector> {
    _path: String,
    pub(crate) entities: Vec<LogEntry>,
    _selector: S,
}

impl LogEntry {
    pub fn get_log_message(&self) -> &str {
        &self.message
    }
}

impl<S: TimestampSelector> AsRef<[LogEntry]> for LogFile<S> {
    fn as_ref(&self) -> &[LogEntry] {
        &self.entities
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
            _path: path,
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
    use crate::application::compare_logfiles;

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
        let result = compare_logfiles(vec![&left, &right]);
        assert_eq!(result.containers()[0], ["left", "extra", "\n"]);
        assert_eq!(result.containers()[1], ["right", "\n", "later"]);
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
        let result = compare_logfiles(vec![left.as_ref(), right.as_ref()]);
        assert_eq!(result.length(), 1);
        assert_eq!(result.containers()[0], ["custom"]);
        assert_eq!(result.containers()[1], ["automatic"]);
    }

    #[test]
    fn colored_logs_parse_and_compare_like_plain_text() {
        let plain = "2026-09-12 00:00:01 ERROR failed\n    RuntimeError: example\n2026-09-12 00:00:02 INFO recovered";
        let colored = "\x1b[32m2026-09-12 00:00:01\x1b[0m \x1b[31mERROR failed\x1b[0m\r\n\t\x1b[1;35mRuntimeError: example\x1b[0m\r\n\x1b[38;2;255;0;0m2026-09-12 00:00:02 INFO recovered\x1b[0m";
        let left = LogFile::from_source(String::new(), plain);
        let right = LogFile::from_source(String::new(), colored);

        assert_eq!(right.entities.len(), 2);
        assert_eq!(left.entities[0].timestamp, right.entities[0].timestamp);
        let result = compare_logfiles(vec![&left, &right]);
        assert_eq!(result.length(), 2);
        assert_eq!(result.containers()[0], result.containers()[1]);
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
            _path: String::new(),
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
        let result = compare_logfiles(vec![&left, &right]);

        assert_eq!(result.containers()[0], ["L1", "\n", "L3", "L4", "\n"]);
        assert_eq!(result.containers()[1], ["\n", "R2", "R3", "\n", "R5"]);
        assert_eq!(result.length(), 5);
        assert_eq!(left.entities.len(), 3);
        assert_eq!(right.entities.len(), 3);
    }

    #[test]
    fn handles_empty_logs_and_remaining_entries_on_either_side() {
        let empty = logfile(&[], "");
        let populated = logfile(&[1, 2], "L");
        let result = compare_logfiles(vec![&empty, &empty]);
        assert!(result.containers()[0].is_empty());
        assert!(result.containers()[1].is_empty());
        assert_eq!(result.length(), 0);

        let result = compare_logfiles(vec![&populated, &empty]);
        assert_eq!(result.containers()[0], ["L1", "L2"]);
        assert_eq!(result.containers()[1], ["\n", "\n"]);
        assert_eq!(result.length(), 2);

        let result = compare_logfiles(vec![&empty, &populated]);
        assert_eq!(result.containers()[0], ["\n", "\n"]);
        assert_eq!(result.containers()[1], ["L1", "L2"]);
        assert_eq!(result.length(), 2);
    }

    #[test]
    fn pairs_duplicate_timestamps_in_order() {
        let result = compare_logfiles(vec![&logfile(&[1, 1], "L"), &logfile(&[1], "R")]);
        assert_eq!(result.containers()[0], ["L1", "L1"]);
        assert_eq!(result.containers()[1], ["R1", "\n"]);
        assert_eq!(result.length(), 2);
    }

    #[test]
    fn aligns_multiple_logs_with_duplicates_and_empty_inputs() {
        let first = logfile(&[1, 3, 3, 6], "A");
        let second = logfile(&[2, 3, 5], "B");
        let third = logfile(&[1, 3, 3, 3, 7], "C");
        let empty = logfile(&[], "");
        let result = compare_logfiles(vec![&first, &second, &third, &empty]);

        assert_eq!(result.length(), 8);
        assert_eq!(
            result.containers()[0],
            ["A1", "\n", "A3", "A3", "\n", "\n", "A6", "\n"]
        );
        assert_eq!(
            result.containers()[1],
            ["\n", "B2", "B3", "\n", "\n", "B5", "\n", "\n"]
        );
        assert_eq!(
            result.containers()[2],
            ["C1", "\n", "C3", "C3", "C3", "\n", "\n", "C7"]
        );
        assert_eq!(result.containers()[3], vec!["\n"; 8]);
        assert_eq!(
            result.timestamps(),
            [1, 2, 3, 3, 3, 5, 6, 7].map(|second| { format!("2026-09-12 00:00:{second:02} UTC") })
        );
    }

    #[test]
    fn compares_zero_single_and_all_empty_logs() {
        let result = compare_logfiles(Vec::<&LogFile>::new());
        assert!(result.containers().is_empty());
        assert!(result.timestamps().is_empty());
        assert_eq!(result.length(), 0);

        let single = logfile(&[1, 2], "A");
        let result = compare_logfiles(vec![&single]);
        assert_eq!(result.containers().len(), 1);
        assert_eq!(result.containers()[0], ["A1", "A2"]);
        assert_eq!(result.length(), 2);

        let empty = logfile(&[], "");
        let result = compare_logfiles(vec![&empty; 4]);
        assert_eq!(result.containers().len(), 4);
        assert!(result.containers().iter().all(Vec::is_empty));
        assert_eq!(result.length(), 0);
    }

    #[test]
    fn preserves_duplicate_message_order_across_three_logs() {
        let first = LogFile::from_source(
            String::new(),
            "2026-09-12 10:00:00 A-first\n2026-09-12 10:00:00 A-second",
        );
        let second = LogFile::from_source(String::new(), "2026-09-12 10:00:00 B-only");
        let third = LogFile::from_source(
            String::new(),
            "2026-09-12 10:00:00 C-first\n2026-09-12 10:00:00 C-second\n2026-09-12 10:00:00 C-third",
        );
        let result = compare_logfiles(vec![&first, &second, &third]);

        assert_eq!(result.length(), 3);
        assert_eq!(result.containers()[0], ["A-first", "A-second", "\n"]);
        assert_eq!(result.containers()[1], ["B-only", "\n", "\n"]);
        assert_eq!(result.containers()[2], ["C-first", "C-second", "C-third"]);
        assert_eq!(result.timestamps(), vec!["2026-09-12 10:00:00 UTC"; 3]);
    }
}
