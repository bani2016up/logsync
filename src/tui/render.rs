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
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(21),
            Constraint::Length(rows[0].width.saturating_sub(21) / 2),
            Constraint::Min(0),
        ])
        .split(rows[0]);
    let mut timestamps = String::new();
    let mut left_text = String::new();
    let mut right_text = String::new();

    for (left, right) in result
        .left_container()
        .iter()
        .zip(result.right_container())
        .skip(offset)
        .take(usize::from(rows[0].height.saturating_sub(2)))
    {
        // Log messages start with a 19-byte timestamp; missing entries are "\n".
        let timestamp = left.get(..19).or_else(|| right.get(..19)).unwrap_or("");
        let left = left.get(19..).unwrap_or("").trim_start();
        let right = right.get(19..).unwrap_or("").trim_start();
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
            vec!["2026-09-12 00:00:01 left\ncontinued".into(), "\n".into()],
            vec![
                "2026-09-12 00:00:01 right".into(),
                "2026-09-12 00:00:02 only-right".into(),
            ],
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
        let result = CompareResult::new(vec![], vec![]).unwrap();
        for (width, height) in [(0, 0), (10, 3), (40, 10)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| draw(frame, &result, 0, 0)).unwrap();
        }
    }

    #[test]
    fn horizontal_scroll_moves_both_messages_but_not_timestamps() {
        let result = CompareResult::new(
            vec!["2026-09-12 00:00:01 abcdef\n123456".into()],
            vec!["2026-09-12 00:00:01 ghijkl".into()],
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
}
