use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use std::borrow::Cow;

/// Component that can cycles through elements
/// while retaining a selected index.
/// It does not own the data it will render,
/// so when rendering it is up to the caller to
/// ensure the coherency between the number of
/// elements this component can select and
/// the number of elements to render in it.
///
/// This choice makes the component agnostic
/// of the actual data, i.e. it does not own it.
/// This is because the actual data may be owned
/// by another entity.
#[derive(Default)]
pub struct Selector {
    state: ListState,
    selection: Option<usize>,
    len: usize,
}

impl Selector {
    pub fn with_len(len: usize) -> Self {
        let mut state = ListState::default();
        state.select(len.ge(&1).then_some(0));

        Self {
            state,
            selection: None,
            len,
        }
    }

    #[allow(unused)]
    pub fn resize(&mut self, new_len: usize, selection: Option<usize>) {
        self.state.select(selection);
        self.selection = selection;
        self.len = new_len;
    }

    pub fn next(&mut self) {
        if self.len == 0 {
            return;
        }

        let next = self.state.selected().map(|i| (i + 1) % self.len);
        self.state.select(next);
    }

    pub fn previous(&mut self) {
        if self.len == 0 {
            return;
        }

        let prev = self.state.selected().map(|i| (i + self.len - 1) % self.len);
        self.state.select(prev);
    }

    pub fn confirm_selection(&mut self) {
        self.selection = self.state.selected();
    }

    pub fn select(&mut self, index: usize) {
        if index < self.len {
            self.state.select(Some(index));
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.selection
    }

    pub fn render<'a, T>(
        &'a mut self,
        f: &mut Frame,
        area: Rect,
        title: &'a str,
        items: &[T],
        is_highlighted: bool,
    ) where
        for<'b> Cow<'b, str>: From<&'b T>,
    {
        let items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let color = if self.selection.is_some() && i == self.selection.unwrap() {
                    Color::Cyan
                } else {
                    Color::Gray
                };

                ListItem::new(Span::styled(item, Style::default().fg(color)))
                    .style(Style::default())
            })
            .collect();

        let border_color = if is_highlighted {
            Color::Gray
        } else {
            Color::DarkGray
        };

        let items = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .fg(border_color)
                    .title(title),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        f.render_stateful_widget(items, area, &mut self.state);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_wrap_around_while_cycling_forward() {
        const LEN: usize = 3;
        let mut selector = Selector::with_len(LEN);
        assert_eq!(selector.selected(), None);
        selector.confirm_selection();

        for i in 0..LEN {
            assert_eq!(selector.selected().unwrap(), i);
            selector.next();
            selector.confirm_selection();
        }

        assert_eq!(selector.selected().unwrap(), 0);
    }

    #[test]
    fn can_wrap_around_while_cycling_backwards() {
        const LEN: usize = 3;
        let mut selector = Selector::with_len(LEN);
        assert_eq!(selector.selected(), None);
        selector.confirm_selection();

        for i in (0..LEN).rev() {
            selector.previous();
            selector.confirm_selection();
            assert_eq!(selector.selected().unwrap(), i);
        }

        assert_eq!(selector.selected().unwrap(), 0);
    }

    #[test]
    fn can_cycle_through_indices_while_retaining_the_selection() {
        let mut selector = Selector::with_len(3);
        assert_eq!(selector.selected(), None);

        selector.next();
        assert_eq!(selector.selected(), None);

        selector.next();
        assert_eq!(selector.selected(), None);

        selector.confirm_selection();
        assert_eq!(selector.selected().unwrap(), 2);

        selector.previous();
        assert_eq!(selector.selected().unwrap(), 2);

        selector.previous();
        assert_eq!(selector.selected().unwrap(), 2);

        selector.confirm_selection();
        assert_eq!(selector.selected().unwrap(), 0);
    }
}
