use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
};

const MAX_NUM_MESSAGES_ON_SCREEN: usize = 64;

#[derive(Default)]
pub struct MidiMessageStream {
    messages: Vec<crate::midi::MidiMessageString>,
}

impl MidiMessageStream {
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn collect(&mut self, mut messages: Vec<crate::midi::MidiMessageString>) {
        if self.messages.len() > MAX_NUM_MESSAGES_ON_SCREEN {
            self.messages = self
                .messages
                .split_off(messages.len().min(self.messages.len() - 1));
        }

        self.messages.append(&mut messages);
    }

    pub fn make_list_view<'a>(&'a self, title: &'a str) -> List<'a> {
        let message_list: Vec<ListItem> = self
            .messages
            .iter()
            .rev()
            .enumerate()
            .map(|(i, msg)| {
                let style = if i == 0 {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(vec![Line::from(vec![
                    Span::styled(format!("[ {} ]", msg.timestamp), style.fg(Color::Gray)),
                    Span::styled(" : ", style.fg(Color::DarkGray)),
                    Span::styled(msg.category.clone(), style.fg(Color::Cyan)),
                    Span::styled(" : ", style.fg(Color::DarkGray)),
                    Span::styled(msg.data.clone(), style.fg(Color::Yellow)),
                ])])
            })
            .collect();

        List::new(message_list)
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::DarkGray))
                    .title(Span::styled(
                        title,
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
            )
    }
}
