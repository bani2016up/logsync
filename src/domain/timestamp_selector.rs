use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use std::sync::LazyLock;

pub(crate) struct SelectedTimestamp<'a> {
    pub(crate) timestamp: DateTime<Utc>,
    pub(crate) message: &'a str,
}

pub(crate) trait TimestampSelector {
    fn select<'a>(&mut self, line: &'a str) -> Option<SelectedTimestamp<'a>>;
}

#[derive(Default)]
pub(crate) struct AutoTimestampSelector;

impl TimestampSelector for AutoTimestampSelector {
    fn select<'a>(&mut self, line: &'a str) -> Option<SelectedTimestamp<'a>> {
        // Match a complete leading timestamp, never a seconds-only prefix of one.
        static PREFIX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(concat!(
                r"^(?:",
                r"\d{4}[-/.]\d{2}[-/.]\d{2}[Tt ]\d{2}:\d{2}:\d{2}",
                r"(?:[.,]\d{1,9})?(?:[ \t]*(?:[Zz]|UTC|GMT|[+-]\d{2}:?\d{2}))?",
                r"|(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun), \d{1,2} [A-Z][a-z]{2} \d{4} ",
                r"\d{2}:\d{2}:\d{2} (?:[+-]\d{4}|GMT|UTC|UT)",
                r"|\d{2}/[A-Z][a-z]{2}/\d{4}:\d{2}:\d{2}:\d{2} [+-]\d{4}",
                r"|-?(?:\d{19}|\d{16}|\d{13}|\d{10})",
                r")"
            ))
            .expect("valid timestamp pattern")
        });

        let line = line.trim_start();
        let bracketed = line.starts_with('[');
        let line = if bracketed { &line[1..] } else { line };
        let matched = PREFIX.find(line)?;
        let timestamp_text = matched.as_str();
        let remainder = &line[matched.end()..];
        let message = if bracketed {
            remainder.strip_prefix(']')?
        } else {
            remainder
        };
        if !message.is_empty() && !message.starts_with(char::is_whitespace) {
            return None;
        }

        let timestamp = if timestamp_text
            .trim_start_matches('-')
            .bytes()
            .all(|b| b.is_ascii_digit())
        {
            let value: i64 = timestamp_text.parse().ok()?;
            let scale = match timestamp_text.trim_start_matches('-').len() {
                10 => 1,
                13 => 1_000,
                16 => 1_000_000,
                19 => 1_000_000_000,
                _ => return None,
            };
            DateTime::from_timestamp(
                value.div_euclid(scale),
                (value.rem_euclid(scale) * (1_000_000_000 / scale)) as u32,
            )?
        } else if let Ok(timestamp) = DateTime::parse_from_rfc2822(timestamp_text) {
            timestamp.with_timezone(&Utc)
        } else if let Ok(timestamp) =
            DateTime::parse_from_str(timestamp_text, "%d/%b/%Y:%H:%M:%S %z")
        {
            timestamp.with_timezone(&Utc)
        } else {
            let normalized = timestamp_text
                .replace("UTC", "+0000")
                .replace("GMT", "+0000")
                .replace(['T', 't'], " ")
                .replace(',', ".")
                .replace(['Z', 'z'], "+0000");
            let mut parsed = None;
            for date in ["%Y-%m-%d", "%Y/%m/%d", "%Y.%m.%d"] {
                for zone in ["%z", " %z"] {
                    if let Ok(timestamp) =
                        DateTime::parse_from_str(&normalized, &format!("{date} %H:%M:%S%.f{zone}"))
                    {
                        parsed = Some(timestamp.with_timezone(&Utc));
                        break;
                    }
                }
                if parsed.is_some() {
                    break;
                }
                if let Ok(timestamp) =
                    NaiveDateTime::parse_from_str(&normalized, &format!("{date} %H:%M:%S%.f"))
                {
                    // Do not silently interpret common unsupported zones as UTC.
                    let suffix = message.trim_start();
                    if !bracketed
                        && ((suffix.starts_with(['+', '-'])
                            && suffix.as_bytes().get(1).is_some_and(u8::is_ascii_digit))
                            || matches!(
                                suffix.split_whitespace().next(),
                                Some(
                                    "EST"
                                        | "EDT"
                                        | "CST"
                                        | "CDT"
                                        | "MST"
                                        | "MDT"
                                        | "PST"
                                        | "PDT"
                                        | "CET"
                                        | "CEST"
                                        | "BST"
                                        | "IST"
                                )
                            ))
                    {
                        return None;
                    }
                    // Zone-less logs use UTC, independent of the machine's local timezone.
                    parsed = Some(timestamp.and_utc());
                    break;
                }
            }
            parsed?
        };

        Some(SelectedTimestamp {
            timestamp,
            message: message.trim_start(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_formats_and_normalizes_offsets() {
        let expected = DateTime::parse_from_rfc3339("2026-09-12T10:20:30Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut selector = AutoTimestampSelector;
        for timestamp in [
            "2026-09-12 10:20:30",
            "2026/09/12 10:20:30",
            "2026.09.12 10:20:30",
            "2026-09-12T10:20:30Z",
            "2026-09-12t10:20:30Z",
            "2026-09-12t10:20:30z",
            "2026-09-12 10:20:30 UTC",
            "2026-09-12 10:20:30 GMT",
            "2026-09-12T12:20:30+02:00",
            "2026-09-12 05:20:30 -0500",
            "2026-09-12 12:20:30  +0200",
            "2026-09-12 12:20:30\t+0200",
            "Sat, 12 Sep 2026 10:20:30 +0000",
            "Sat, 12 Sep 2026 10:20:30 GMT",
            "12/Sep/2026:12:20:30 +0200",
        ] {
            for line in [
                format!("{timestamp} INFO hello"),
                format!("[{timestamp}] INFO hello"),
            ] {
                let selected = selector
                    .select(&line)
                    .unwrap_or_else(|| panic!("not detected: {line}"));
                assert_eq!(selected.timestamp, expected, "{line}");
                assert_eq!(selected.message, "INFO hello");
            }
        }
    }

    #[test]
    fn preserves_fractional_seconds() {
        for fraction in ["1", "123", "123456", "123456789"] {
            let expected =
                DateTime::parse_from_rfc3339(&format!("2026-09-12T10:20:30.{fraction}Z"))
                    .unwrap()
                    .with_timezone(&Utc);
            for separator in ['.', ','] {
                let line = format!("2026-09-12 10:20:30{separator}{fraction} message");
                let selected = AutoTimestampSelector.select(&line).unwrap();
                assert_eq!(selected.timestamp, expected);
                assert_eq!(selected.message, "message");
            }
        }
        let selected = AutoTimestampSelector
            .select("2026-09-12T12:20:30.123456789+02:00 message")
            .unwrap();
        assert_eq!(
            selected.timestamp.to_rfc3339(),
            "2026-09-12T10:20:30.123456789+00:00"
        );
    }

    #[test]
    fn detects_unix_seconds_milliseconds_microseconds_and_nanoseconds() {
        for (text, seconds, nanos) in [
            ("1700000000", 1_700_000_000, 0),
            ("1700000000123", 1_700_000_000, 123_000_000),
            ("1700000000123456", 1_700_000_000, 123_456_000),
            ("1700000000123456789", 1_700_000_000, 123_456_789),
            ("-1700000000123", -1_700_000_001, 877_000_000),
        ] {
            let selected = AutoTimestampSelector.select(text).unwrap();
            assert_eq!(
                selected.timestamp,
                DateTime::from_timestamp(seconds, nanos).unwrap()
            );
            assert_eq!(selected.message, "");
        }
    }

    #[test]
    fn rejects_invalid_ambiguous_and_partial_timestamps() {
        for line in [
            "",
            "Traceback:",
            "12:20:30 INFO",
            "01/02/2026 12:20:30 INFO",
            "2026-02-30 12:20:30 INFO",
            "2026-09-12T10:20:30+25:00 INFO",
            "2026-09-12 10:20:30 +02 INFO",
            "2026-09-12 10:20:30 EST INFO",
            "2026-09-12 10:20:30 CST INFO",
            "2026-09-12 10:20:30  +25:00 INFO",
            "2026-09-12T10:20:30.1234567890 INFO",
            "17000000001234567890 INFO",
            "2026-09-12T10:20:30Zbroken",
            "[2026-09-12 10:20:30 INFO",
            "2026-09-12 10:20:30. INFO",
            "prefix 2026-09-12 10:20:30 INFO",
        ] {
            assert!(
                AutoTimestampSelector.select(line).is_none(),
                "unexpected match: {line}"
            );
        }
    }
}
