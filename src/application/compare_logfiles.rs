use crate::domain::{Compare, CompareResult, LogFile, TimestampSelector};

/// Aligns logs sorted by timestamp, using a newline for missing entries.
pub(crate) fn compare_logfiles<L: TimestampSelector, R: TimestampSelector>(
    logfile1: &LogFile<L>,
    logfile2: &LogFile<R>,
) -> CompareResult {
    let mut left_container = Vec::new();
    let mut right_container = Vec::new();
    let mut timestamps: Vec<String> = Vec::new();
    let mut left = logfile1.entities.iter().peekable();
    let mut right = logfile2.entities.iter().peekable();

    loop {
        let (take_left, take_right) = match (left.peek(), right.peek()) {
            (Some(entry1), Some(entry2)) => {
                (entry1.key() <= entry2.key(), entry1.key() >= entry2.key())
            }
            (Some(_), None) => (true, false),
            (None, Some(_)) => (false, true),
            (None, None) => break,
        };

        let entry = if take_left {
            left.peek().unwrap()
        } else {
            right.peek().unwrap()
        };
        timestamps.push(
            entry
                .timestamp
                .format("%Y-%m-%d %H:%M:%S%.f UTC")
                .to_string(),
        );

        left_container.push(if take_left {
            left.next().unwrap().get_log_message().to_owned()
        } else {
            String::from("\n")
        });
        right_container.push(if take_right {
            right.next().unwrap().get_log_message().to_owned()
        } else {
            String::from("\n")
        });
    }

    CompareResult::new(left_container, right_container, timestamps)
        .expect("comparison appends one entry to each column per iteration")
}
