use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
};

const USAGE: &str = r#"
         ? : display help
   <SPACE> : pause / resume
   <UP>, k : scroll up
 <DOWN>, j : scroll down
     Enter : confirm selection
  <ESC>, q : quit or hide help
     <C-c> : force quit
"#;

pub fn render<B: Backend>(f: &mut Frame<B>, app: &mut super::app::App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(3), Constraint::Percentage(80)].as_ref())
        .split(f.size());

    app.port_names.render_selector(f, sections[0], "˧ ports ꜔");

    let message_list: Vec<ListItem> = app
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

    let selected_port_name = match app.selection.clone() {
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

    f.render_widget(message_view, sections[1]);

    if app.show_usage {
        crate::widgets::usage::render(f, USAGE);
    }
}
