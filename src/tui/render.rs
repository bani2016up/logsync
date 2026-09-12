use crate::domain::CompareResult;
use tui::{
    Frame,
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
};

pub(super) fn draw<B: Backend>(
    frame: &mut Frame<B>,
    result: &CompareResult,
    offset: usize,
    horizontal_offset: u16,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.size());
    let timestamp_width = result
        .max_timestamp_length()
        .max("Timestamp".len())
        .saturating_add(2)
        .min(usize::from(rows[0].width / 2)) as u16;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(timestamp_width),
            Constraint::Length(rows[0].width.saturating_sub(timestamp_width) / 2),
            Constraint::Min(0),
        ])
        .split(rows[0]);
    let mut timestamps = String::new();
    let mut left_text = String::new();
    let mut right_text = String::new();

    for ((left, right), timestamp) in result
        .left_container()
        .iter()
        .zip(result.right_container())
        .zip(result.timestamps())
        .skip(offset)
        .take(usize::from(rows[0].height.saturating_sub(2)))
    {
        let height = left.lines().count().max(right.lines().count()).max(1);

        // Pad multiline entries so the next pair starts on the same screen row.
        timestamps.push_str(timestamp);
        timestamps.push_str(&"\n".repeat(height));
        let mut left_lines = left.lines();
        let mut right_lines = right.lines();
        for _ in 0..height {
            left_text.push_str(left_lines.next().unwrap_or(""));
            left_text.push('\n');
            right_text.push_str(right_lines.next().unwrap_or(""));
            right_text.push('\n');
        }
    }

    for ((title, text, scroll), area) in [
        ("Timestamp", timestamps, 0),
        ("Left", left_text, horizontal_offset),
        ("Right", right_text, horizontal_offset),
    ]
    .into_iter()
    .zip(columns.iter())
    {
        frame.render_widget(
            Paragraph::new(text)
                .scroll((0, scroll))
                .block(Block::default().title(title).borders(Borders::ALL)),
            *area,
        );
    }
    let help = if result.length() == 0 {
        "No log entries | q / Esc: quit"
    } else {
        "Up/Down: vertical | Left/Right: horizontal | q / Esc: quit"
    };
    frame.render_widget(Paragraph::new(help), rows[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui::{Terminal, backend::TestBackend};

    #[test]
    fn renders_aligned_columns_and_scrolls() {
        let result = CompareResult::new(
            vec!["left\ncontinued".into(), "\n".into()],
            vec!["right".into(), "only-right".into()],
            vec!["2026-09-12 00:00:01".into(), "2026-09-12 00:00:02".into()],
        )
        .unwrap();
        let mut terminal = Terminal::new(TestBackend::new(100, 10)).unwrap();
        terminal.draw(|frame| draw(frame, &result, 0, 0)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.get(1, 1).symbol, "2");
        assert_eq!(buffer.get(22, 1).symbol, "l");
        assert_eq!(buffer.get(22, 2).symbol, "c");
        assert_eq!(buffer.get(61, 1).symbol, "r");
        assert_eq!(buffer.get(61, 2).symbol, " ");
        assert_eq!(buffer.get(61, 3).symbol, "o");

        terminal.draw(|frame| draw(frame, &result, 1, 0)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.get(19, 1).symbol, "2");
        assert_eq!(buffer.get(22, 1).symbol, " ");
        assert_eq!(buffer.get(61, 1).symbol, "o");
    }

    #[test]
    fn renders_empty_results_in_small_terminals() {
        let result = CompareResult::new(vec![], vec![], vec![]).unwrap();
        for (width, height) in [(0, 0), (10, 3), (40, 10)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| draw(frame, &result, 0, 0)).unwrap();
        }
    }

    #[test]
    fn horizontal_scroll_moves_both_messages_but_not_timestamps() {
        let result = CompareResult::new(
            vec!["abcdef\n123456".into()],
            vec!["ghijkl".into()],
            vec!["2026-09-12 00:00:01".into()],
        )
        .unwrap();
        let mut terminal = Terminal::new(TestBackend::new(100, 10)).unwrap();
        terminal.draw(|frame| draw(frame, &result, 0, 3)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.get(1, 1).symbol, "2");
        assert_eq!(buffer.get(19, 1).symbol, "1");
        assert_eq!(buffer.get(22, 1).symbol, "d");
        assert_eq!(buffer.get(22, 2).symbol, "4");
        assert_eq!(buffer.get(61, 1).symbol, "j");
        assert_eq!(buffer.get(61, 2).symbol, " ");
    }

    #[test]
    fn renders_variable_length_timestamps_and_caps_width() {
        let timestamps = vec![
            "2026-09-12 00:00:01 UTC".to_owned(),
            "2026-09-12 00:00:02.123456789 UTC".to_owned(),
        ];
        let result = CompareResult::new(
            vec!["left".into(), "  indented".into()],
            vec!["right".into(), "\n".into()],
            timestamps.clone(),
        )
        .unwrap();
        let mut terminal = Terminal::new(TestBackend::new(100, 10)).unwrap();
        terminal.draw(|frame| draw(frame, &result, 0, 0)).unwrap();
        let buffer = terminal.backend().buffer();
        for (row, timestamp) in timestamps.iter().enumerate() {
            for (column, character) in timestamp.chars().enumerate() {
                assert_eq!(
                    buffer.get(column as u16 + 1, row as u16 + 1).symbol,
                    character.to_string()
                );
            }
        }
        let left_start = timestamps[1].chars().count() as u16 + 3;
        assert_eq!(buffer.get(left_start, 1).symbol, "l");
        assert_eq!(buffer.get(left_start, 2).symbol, " ");
        assert_eq!(buffer.get(left_start + 2, 2).symbol, "i");

        let mut terminal = Terminal::new(TestBackend::new(40, 6)).unwrap();
        terminal.draw(|frame| draw(frame, &result, 0, 0)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.get(21, 1).symbol, "l");
        assert_eq!(buffer.get(31, 1).symbol, "r");
    }
}
