mod render;
mod state;

use crate::domain::CompareResult;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use state::State;
use std::io;
use tui::{Terminal, backend::CrosstermBackend};

pub(crate) fn start(result: &CompareResult) -> io::Result<()> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    enable_raw_mode()?;

    // Keep errors inside the closure so terminal cleanup still runs.
    let run_result = (|| -> io::Result<()> {
        execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        terminal.hide_cursor()?;
        let mut state = State::default();

        loop {
            terminal.draw(|frame| {
                render::draw(frame, result, state.offset(), state.horizontal_offset())
            })?;
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up => state.scroll_up(),
                    KeyCode::Down => state.scroll_down(result.length()),
                    KeyCode::Left => state.scroll_left(),
                    KeyCode::Right => state.scroll_right(),
                    _ => {}
                },
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollLeft => state.scroll_left(),
                    MouseEventKind::ScrollRight => state.scroll_right(),
                    // Some terminals report Shift + wheel instead of horizontal scrolling.
                    MouseEventKind::ScrollUp if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
                        state.scroll_left();
                    }
                    MouseEventKind::ScrollDown if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
                        state.scroll_right();
                    }
                    MouseEventKind::ScrollUp => state.scroll_up(),
                    MouseEventKind::ScrollDown => state.scroll_down(result.length()),
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(())
    })();

    let raw_result = disable_raw_mode();
    let mouse_result = execute!(terminal.backend_mut(), DisableMouseCapture);
    let screen_result = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let cursor_result = terminal.show_cursor();
    run_result
        .and(raw_result)
        .and(mouse_result)
        .and(screen_result)
        .and(cursor_result)
}
