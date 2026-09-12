#[derive(Default)]
pub(super) struct State {
    offset: usize,
    horizontal_offset: u16,
}

impl State {
    pub(super) fn offset(&self) -> usize {
        self.offset
    }

    pub(super) fn horizontal_offset(&self) -> u16 {
        self.horizontal_offset
    }

    pub(super) fn scroll_left(&mut self) {
        self.horizontal_offset = self.horizontal_offset.saturating_sub(1);
    }

    pub(super) fn scroll_right(&mut self) {
        self.horizontal_offset = self.horizontal_offset.saturating_add(1);
    }

    pub(super) fn scroll_up(&mut self) {
        self.offset = self.offset.saturating_sub(1);
    }

    pub(super) fn scroll_down(&mut self, length: usize) {
        self.offset = self.offset.saturating_add(1).min(length.saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::State;

    #[test]
    fn horizontal_scrolling_saturates_and_preserves_vertical_offset() {
        let mut state = State::default();
        state.scroll_down(3);
        state.scroll_left();
        assert_eq!(state.horizontal_offset(), 0);
        state.scroll_right();
        assert_eq!(state.horizontal_offset(), 1);
        state.scroll_left();
        assert_eq!(state.horizontal_offset(), 0);
        state.horizontal_offset = u16::MAX;
        state.scroll_right();
        assert_eq!(state.horizontal_offset(), u16::MAX);
        assert_eq!(state.offset(), 1);
    }

    #[test]
    fn scrolling_stays_within_bounds() {
        let mut state = State::default();
        state.scroll_up();
        assert_eq!(state.offset(), 0);
        state.scroll_down(0);
        assert_eq!(state.offset(), 0);
        state.scroll_down(2);
        assert_eq!(state.offset(), 1);
        state.scroll_down(2);
        assert_eq!(state.offset(), 1);
        state.scroll_up();
        assert_eq!(state.offset(), 0);
    }
}
