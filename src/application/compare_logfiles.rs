use crate::domain::{Compare, CompareResult, LogEntry};

/// Aligns sorted logs, pairing duplicate timestamps in encounter order.
pub(crate) fn compare_logfiles(logfiles: Vec<impl AsRef<[LogEntry]>>) -> CompareResult {
    let mut containers = vec![Vec::new(); logfiles.len()];
    let mut timestamps = Vec::new();
    let mut entries: Vec<_> = logfiles
        .iter()
        .map(|file| file.as_ref().iter().peekable())
        .collect();

    while let Some(timestamp) = entries
        .iter_mut()
        .filter_map(|entries| entries.peek().map(|entry| entry.key()))
        .min()
    {
        timestamps.push(timestamp.format("%Y-%m-%d %H:%M:%S%.f UTC").to_string());

        // Every row has one cell per log, even for missing or exhausted inputs.
        for (entries, container) in entries.iter_mut().zip(&mut containers) {
            if entries.peek().is_some_and(|entry| entry.key() == timestamp) {
                container.push(entries.next().unwrap().get_log_message().to_owned());
            } else {
                container.push(String::from("\n"));
            }
        }
    }

    CompareResult::new(containers, timestamps)
        .expect("comparison appends one entry to every column per timestamp")
}
