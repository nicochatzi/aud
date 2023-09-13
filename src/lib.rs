use std::borrow::Cow;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

pub mod terminal {
    use super::*;
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    };

    type StdoutTerminal = Terminal<CrosstermBackend<std::io::Stdout>>;

    pub fn acquire() -> anyhow::Result<StdoutTerminal> {
        crossterm::terminal::enable_raw_mode()?;

        let mut stdout = std::io::stdout();
        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = backend::CrosstermBackend::new(stdout);
        Ok(Terminal::new(backend)?)
    }

    pub fn release(mut terminal: StdoutTerminal) -> anyhow::Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        Ok(())
    }
}

#[macro_export]
macro_rules! main {
    ($app_entry: expr) => {
        pub fn main() -> anyhow::Result<()> {
            let mut terminal = $crate::terminal::acquire()?;
            let app_result = $app_entry(&mut terminal);
            $crate::terminal::release(terminal)?;
            if let Err(e) = app_result {
                eprintln!("{e}");
            }
            Ok(())
        }
    };
}

#[derive(Default)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        let mut this = Self {
            state: ListState::default(),
            items,
        };
        if !this.items.is_empty() {
            this.state.select(Some(0));
        }
        this
    }

    pub fn next(&mut self) {
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
        frame: &mut Frame<B>,
        area: Rect,
        title: &'a str,
    ) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| {
                ListItem::new(Span::styled(item, Style::default().fg(Color::Gray)))
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
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(items, area, &mut self.state);
    }
}
