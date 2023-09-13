use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, Paragraph},
};

pub fn render<B: Backend>(frame: &mut Frame<B>, state: &crate::state::AppState) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Max(8), Constraint::Percentage(20)].as_ref())
        .split(frame.size());
    let top_sections = Layout::default()
        .direction(Direction::Horizontal)
        .margin(0)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(sections[0]);

    let is_on_beat = state.beats() % 1. < 0.25;
    let (beat_color, block_color) = if state.is_playing() && is_on_beat {
        (Color::Yellow, Color::Gray)
    } else {
        (Color::Cyan, Color::DarkGray)
    };
    let create_block = |title| {
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(block_color))
            .title(Span::styled(
                title,
                Style::default().add_modifier(Modifier::BOLD),
            ))
    };

    let usage_text = vec![
        Line::from("a     : enable Link"),
        Line::from("s     : enable sync"),
        Line::from("space : start / stop"),
        Line::from("k / j : + / - tempo"),
        Line::from("l / h : + / - quantum"),
        Line::from("q     : quit"),
    ];
    let usage = Paragraph::new(usage_text)
        .style(Style::default().fg(Color::Yellow))
        .block(create_block("˧ usage ꜔"))
        .alignment(Alignment::Left);
    frame.render_widget(usage, top_sections[1]);

    let status_text = vec![
        Line::from(format!("peers   : {}", state.num_peers())),
        Line::from(format!("sync    : {}", state.is_start_stop_sync_enabled())),
        Line::from(format!("state   : {}", state.is_playing())),
        Line::from(format!("tempo   : {:<3.2}", state.tempo())),
        Line::from(format!("beats   : {:<8.2}", state.beats())),
        Line::from(format!("quantum : {}", state.quantum())),
    ];
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(beat_color))
        .block(create_block("˧ status ꜔"))
        .alignment(Alignment::Left);
    frame.render_widget(status, top_sections[0]);

    let progress = state.beats() % state.quantum();
    let beat_gauge_title = format!(
        "˧ {} : {} : {} ꜔",
        (state.beats() as u64) + 1,
        progress as u8 + 1,
        ((progress % 1.0) * 2.5) as u8 + 1
    );
    let beat_gauge = Gauge::default()
        .block(create_block(&beat_gauge_title))
        .gauge_style(Style::default().fg(beat_color))
        .percent((progress * (100. / state.quantum())) as u16 + 1)
        .label("");
    frame.render_widget(beat_gauge, sections[1]);
}
