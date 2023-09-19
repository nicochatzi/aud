use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
};

pub fn render_messages(
    f: &mut Frame<impl Backend>,
    title: &str,
    messages: &[crate::midi::MidiMessageString],
    area: Rect,
) {
    const MAX_NUM_MESSAGES_ON_SCREEN: usize = 128;

    let message_list: Vec<ListItem> = messages
        .iter()
        .rev()
        .enumerate()
        .take(MAX_NUM_MESSAGES_ON_SCREEN.min(messages.len()))
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

    let list = List::new(message_list)
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(
                    title,
                    Style::default().add_modifier(Modifier::BOLD),
                )),
        );

    f.render_widget(list, area);
}
