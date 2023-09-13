use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render<B: Backend>(frame: &mut Frame<B>, state: &mut crate::State) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(3), Constraint::Percentage(80)].as_ref())
        .split(frame.size());

    state
        .port_names
        .render_selector(frame, sections[0], "˧ ports ꜔");

    let message_list: Vec<ListItem> = state
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

    let selected_port_name = match state.selection.clone() {
        Some(name) => format!("˧ {name} ꜔"),
        None => "".to_owned(),
    };

    let message_view = List::new(message_list)
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(
                    selected_port_name,
                    Style::default().add_modifier(Modifier::BOLD),
                )),
        );

    frame.render_widget(message_view, sections[1]);
}
