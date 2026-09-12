use crate::domain::{Compare, CompareResult, LogFile};

/// Aligns logs sorted by timestamp, using a newline for missing entries.
pub(crate) fn compare_logfiles(logfile1: &LogFile, logfile2: &LogFile) -> CompareResult {
    let mut left_container = Vec::new();
    let mut right_container = Vec::new();
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

    CompareResult::new(left_container, right_container)
        .expect("comparison appends one entry to each column per iteration")
}
