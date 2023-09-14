use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, Paragraph},
};

const USAGE: &str = r#"
        ? : display help
        a : enable / disable Link
        s : enable / disable sync
    space : resume / pause
    k / j : + / - tempo
    l / h : + / - quantum
 <ESC>, q : quit or hide help
    <C-c> : force quit
"#;

pub fn render<B: Backend>(f: &mut Frame<B>, app: &mut super::app::App) {
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Min(12), Constraint::Percentage(70)].as_ref())
        .split(f.size());

    let is_on_beat = app.beats() % 1. < 0.25;
    let (beat_color, block_color) = if app.is_playing() && is_on_beat {
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

    let status_text = vec![
        Line::from(format!("peers   : {}", app.num_peers())),
        Line::from(format!("sync    : {}", app.is_start_stop_sync_enabled())),
        Line::from(format!("state   : {}", app.is_playing())),
        Line::from(format!("tempo   : {:<3.2}", app.tempo())),
        Line::from(format!("beats   : {:<8.2}", app.beats())),
        Line::from(format!("quantum : {}", app.quantum())),
    ];
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(beat_color))
        .block(create_block("˧ status ꜔"))
        .alignment(Alignment::Left);
    f.render_widget(status, sections[0]);

    let progress = app.beats() % app.quantum();
    let beat_gauge_title = format!(
        "˧ {} : {} : {} ꜔",
        (app.beats() as u64) + 1,
        progress as u8 + 1,
        ((progress % 1.0) * 2.5) as u8 + 1
    );
    let beat_gauge = Gauge::default()
        .block(create_block(&beat_gauge_title))
        .gauge_style(Style::default().fg(beat_color))
        .percent((progress * (100. / app.quantum())) as u16 + 1)
        .label("");
    f.render_widget(beat_gauge, sections[1]);

    if app.show_usage {
        crate::widgets::usage::render(f, USAGE);
    }
}
