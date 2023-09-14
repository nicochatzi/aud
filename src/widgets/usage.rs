use ratatui::{prelude::*, widgets::*};

pub fn render<B: Backend>(f: &mut Frame<B>, text: &str) {
    let block = Block::default()
        .title("˧ usage ꜔")
        .borders(Borders::ALL)
        .set_style(Style::default().gray());

    let lines: Vec<_> = text.split('\n').filter(|line| !line.is_empty()).collect();
    let num_lines = lines.len();
    let max_width = lines.iter().fold(0, |max, line| line.len().max(max));
    let lines: Vec<_> = lines
        .iter()
        .map(|line| Line::from(line.to_string()))
        .collect();

    let usage = Paragraph::new(lines)
        .style(Style::default().fg(Color::Yellow))
        .block(block)
        .alignment(Alignment::Left);

    let area = centered_rect(num_lines, max_width, f.size());

    f.render_widget(Clear, area); //this clears out the background
    f.render_widget(usage, area);
}

fn centered_rect(num_lines: usize, max_width: usize, r: Rect) -> Rect {
    let percent_x = 50;
    let percent_y = 25;

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Min(num_lines as u16 + 2),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Min(max_width as u16),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
