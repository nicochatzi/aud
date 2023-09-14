use std::borrow::Cow;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

#[derive(Default)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
    pub selection: Option<usize>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        let mut state = ListState::default();
        state.select(items.len().ge(&1).then_some(0));

        Self {
            state,
            items,
            selection: None,
        }
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
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
        if self.items.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
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

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }
}

impl<T> StatefulList<T>
where
    for<'a> Cow<'a, str>: From<&'a T>,
{
    pub fn render_selector<'a, B: Backend>(
        &'a mut self,
        f: &mut Frame<B>,
        area: Rect,
        title: &'a str,
    ) {
        let items: Vec<ListItem> = self
            .items
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

        let items = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .fg(Color::DarkGray)
                    .title(title),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        f.render_stateful_widget(items, area, &mut self.state);
    }
}
