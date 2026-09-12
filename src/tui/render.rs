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
    let containers = result.containers();
    let available_width = usize::from(rows[0].width.saturating_sub(timestamp_width));
    let count = containers.len();
    let mut constraints = vec![Constraint::Length(timestamp_width)];
    // Zero-width panes need no layout variables, even with more than u16::MAX logs.
    let visible_count = count.min(available_width);
    if count == 0 {
        constraints.push(Constraint::Min(0));
    } else {
        constraints.extend((0..visible_count).map(|index| {
            let width = available_width / count + usize::from(index < available_width % count);
            Constraint::Length(width as u16)
        }));
    }
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(rows[0]);
    let mut timestamps = String::new();
    let mut texts = vec![String::new(); visible_count];

    for (index, timestamp) in result
        .timestamps()
        .iter()
        .enumerate()
        .skip(offset)
        .take(usize::from(rows[0].height.saturating_sub(2)))
    {
        let height = containers
            .iter()
            .map(|container| container[index].lines().count())
            .max()
            .unwrap_or(0)
            .max(1);

        // Pad to the tallest entry across all logs, including zero-width panes.
        timestamps.push_str(timestamp);
        timestamps.push_str(&"\n".repeat(height));
        for (text, container) in texts.iter_mut().zip(containers) {
            let mut lines = container[index].lines();
            for _ in 0..height {
                text.push_str(lines.next().unwrap_or(""));
                text.push('\n');
            }
        }
    }

    frame.render_widget(
        Paragraph::new(timestamps).block(Block::default().title("Timestamp").borders(Borders::ALL)),
        columns[0],
    );
    for (index, (text, area)) in texts.into_iter().zip(columns.iter().skip(1)).enumerate() {
        frame.render_widget(
            Paragraph::new(text).scroll((0, horizontal_offset)).block(
                Block::default()
                    .title(format!("Log {}", index + 1))
                    .borders(Borders::ALL),
            ),
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
            vec![
                vec!["left\ncontinued".into(), "\n".into()],
                vec!["right".into(), "only-right".into()],
            ],
            vec!["2026-09-12 00:00:01".into(), "2026-09-12 00:00:02".into()],
        )
        .unwrap();
        let mut terminal = Terminal::new(TestBackend::new(99, 10)).unwrap();
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
            if width == 40 {
                let buffer = terminal.backend().buffer();
                assert_eq!(buffer.get(1, 0).symbol, "T");
                assert_eq!(buffer.get(12, 0).symbol, " ");
                for (column, character) in "No log entries".chars().enumerate() {
                    assert_eq!(buffer.get(column as u16, 9).symbol, character.to_string());
                }
            }
        }
    }

    #[test]
    fn horizontal_scroll_moves_both_messages_but_not_timestamps() {
        let result = CompareResult::new(
            vec![vec!["abcdef\n123456".into()], vec!["ghijkl".into()]],
            vec!["2026-09-12 00:00:01".into()],
        )
        .unwrap();
        let mut terminal = Terminal::new(TestBackend::new(99, 10)).unwrap();
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
            vec![
                vec!["left".into(), "  indented".into()],
                vec!["right".into(), "\n".into()],
            ],
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

    #[test]
    fn aligns_three_logs_with_uneven_multiline_and_missing_entries() {
        let result = CompareResult::new(
            vec![
                vec!["alpha\ncontinued".into(), "\n".into()],
                vec!["\n".into(), "bravo".into()],
                vec!["charlie\ndelta\necho".into(), "foxtrot".into()],
            ],
            vec!["t1".into(), "t2".into()],
        )
        .unwrap();
        // Timestamp: 11 cells; each log: 20 cells.
        let mut terminal = Terminal::new(TestBackend::new(71, 10)).unwrap();
        terminal.draw(|frame| draw(frame, &result, 0, 0)).unwrap();
        let buffer = terminal.backend().buffer();
        for (x, title) in [(12, "Log 1"), (32, "Log 2"), (52, "Log 3")] {
            for (column, character) in title.chars().enumerate() {
                assert_eq!(
                    buffer.get(x + column as u16, 0).symbol,
                    character.to_string()
                );
            }
        }
        for (x, expected) in [
            (1, ["t", " ", " ", "t"]),
            (12, ["a", "c", " ", " "]),
            (32, [" ", " ", " ", "b"]),
            (52, ["c", "d", "e", "f"]),
        ] {
            for (row, symbol) in expected.iter().enumerate() {
                assert_eq!(buffer.get(x, row as u16 + 1).symbol, *symbol);
            }
        }

        terminal.draw(|frame| draw(frame, &result, 1, 0)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.get(2, 1).symbol, "2");
        assert_eq!(buffer.get(12, 1).symbol, " ");
        assert_eq!(buffer.get(32, 1).symbol, "b");
        assert_eq!(buffer.get(52, 1).symbol, "f");

        terminal.draw(|frame| draw(frame, &result, 0, 3)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.get(1, 1).symbol, "t");
        assert_eq!(buffer.get(2, 4).symbol, "2");
        assert_eq!(buffer.get(12, 1).symbol, "h");
        assert_eq!(buffer.get(12, 2).symbol, "t");
        assert_eq!(buffer.get(32, 1).symbol, " ");
        assert_eq!(buffer.get(32, 4).symbol, "v");
        assert_eq!(buffer.get(52, 1).symbol, "r");
        assert_eq!(buffer.get(52, 2).symbol, "t");
        assert_eq!(buffer.get(52, 3).symbol, "o");

        terminal.draw(|frame| draw(frame, &result, 1, 3)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.get(2, 1).symbol, "2");
        assert_eq!(buffer.get(12, 1).symbol, " ");
        assert_eq!(buffer.get(32, 1).symbol, "v");
        assert_eq!(buffer.get(52, 1).symbol, "t");
    }

    #[test]
    fn renders_one_log_using_all_remaining_width() {
        let result =
            CompareResult::new(vec![vec!["abcdefghijklmnopqr".into()]], vec!["t1".into()]).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(31, 5)).unwrap();
        terminal.draw(|frame| draw(frame, &result, 0, 0)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.get(1, 1).symbol, "t");
        assert_eq!(buffer.get(12, 1).symbol, "a");
        assert_eq!(buffer.get(29, 1).symbol, "r");
    }

    #[test]
    fn handles_many_logs_and_tiny_terminals() {
        for count in [0, 1, 3, 100, usize::from(u16::MAX) + 1] {
            let timestamps = if count == 0 { vec![] } else { vec!["t".into()] };
            let result =
                CompareResult::new(vec![vec!["message".into()]; count], timestamps).unwrap();
            for (width, height) in [(0, 0), (0, 5), (5, 0), (1, 1), (2, 3), (9, 5)] {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                for (offset, horizontal_offset) in [(0, 0), (0, u16::MAX), (usize::MAX, 0)] {
                    terminal
                        .draw(|frame| draw(frame, &result, offset, horizontal_offset))
                        .unwrap();
                }
            }
        }
    }

    #[test]
    fn zero_width_logs_still_determine_row_height() {
        let mut containers = vec![vec!["\n".into(), "\n".into()]; 30];
        containers[29][0] = "one\ntwo\nthree".into();
        let result = CompareResult::new(containers, vec!["t1".into(), "t2".into()]).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(31, 7)).unwrap();
        terminal.draw(|frame| draw(frame, &result, 0, 0)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.get(2, 1).symbol, "1");
        assert_eq!(buffer.get(1, 2).symbol, " ");
        assert_eq!(buffer.get(1, 3).symbol, " ");
        assert_eq!(buffer.get(2, 4).symbol, "2");
    }
}
