use std::borrow::Cow;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

/// List that does not own the data
/// it's listing. It's up to the
/// consumer to provide the range
/// and a view of the elements
/// when it's time to render.
#[derive(Default)]
pub struct ListView {
    state: ListState,
    selection: Option<usize>,
    len: usize,
}

impl ListView {
    pub fn with_len(len: usize) -> Self {
        let mut state = ListState::default();
        state.select(len.ge(&1).then_some(0));

        Self {
            state,
            selection: None,
            len,
        }
    }

    pub fn resize(&mut self, new_len: usize, selection: Option<usize>) {
        self.state.select(selection);
        self.selection = selection;
        self.len = new_len;
    }

    pub fn next(&mut self) {
        if self.len == 0 {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.len == 0 {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.state.select(Some(i));
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
        self.state.selected()
    }
}

impl ListView {
    pub fn render_selector<'a, B, T>(
        &'a mut self,
        f: &mut Frame<B>,
        area: Rect,
        title: &'a str,
        items: &[T],
        is_highlighted: bool,
    ) where
        for<'b> Cow<'b, str>: From<&'b T>,
        B: Backend,
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

/// ListView that couples the data
/// to the UI widget.
#[derive(Default)]
pub struct OwnedList<T> {
    pub list: ListView,
    pub items: Vec<T>,
}

impl<T> OwnedList<T> {
    pub fn with_items(items: Vec<T>) -> Self {
        Self {
            list: ListView::with_len(items.len()),
            items,
        }
    }

    pub fn selected_item(&self) -> Option<&T> {
        self.items.get(self.list.selected()?)
    }
}

impl<T> OwnedList<T>
where
    for<'a> Cow<'a, str>: From<&'a T>,
{
    pub fn render_selector<'a, B: Backend>(
        &'a mut self,
        f: &mut Frame<B>,
        area: Rect,
        title: &'a str,
        is_highlighted: bool,
    ) {
        self.list
            .render_selector(f, area, title, &self.items, is_highlighted);
    }
}
