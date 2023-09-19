use ratatui::{prelude::*, widgets::*};
use syntect::{easy, highlighting, parsing, util};

pub fn render_code<B: Backend>(f: &mut Frame<B>, title: &str, code_text: &str) {
    // TODO: Load these once at the start of your program
    let ps = parsing::SyntaxSet::load_defaults_newlines();
    let ts = highlighting::ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension("lua").unwrap();
    let mut h = easy::HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    let (block, area) = setup_popup(f, title, 80, 80);

    f.render_widget(Clear, area);
    f.render_widget(
        get_highlighted_code(&mut h, code_text, &ps).block(block),
        area,
    );
}

pub fn render_text<B: Backend>(f: &mut Frame<B>, title: &str, text: &str) {
    let lines: Vec<_> = text.split('\n').filter(|l| !l.is_empty()).collect();
    let num_lines = lines.len();
    let max_width = lines.iter().fold(0, |max, line| line.len().max(max));

    let lines: Vec<_> = lines
        .iter()
        .map(|line| Line::from(line.to_string()))
        .collect();

    let text = Paragraph::new(lines)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Left);

    const MARGIN: usize = 4;
    let h = f.size().height as f32;
    let w = f.size().width as f32;

    let (block, area) = setup_popup(
        f,
        title,
        (100. * ((num_lines + MARGIN) as f32 / h)) as u16,
        (100. * ((max_width + MARGIN) as f32 / w)) as u16,
    );

    f.render_widget(Clear, area);
    f.render_widget(text.block(block), area);
}

fn get_highlighted_code<'a>(
    h: &'a mut easy::HighlightLines,
    code: &'a str,
    ps: &'a parsing::SyntaxSet,
) -> List<'a> {
    let mut highlighted_text: Vec<ListItem> = vec![];

    for line in util::LinesWithEndings::from(code) {
        let mut spans: Vec<Span> = vec![];

        let leading_whitespaces = line.chars().take_while(|&c| c == ' ').count();
        let leading_tabs = line.chars().take_while(|&c| c == '\t').count();
        spans.push(Span::raw(
            " ".repeat(leading_whitespaces + (4 * leading_tabs)),
        ));

        let ranges: Vec<(highlighting::Style, &str)> = h.highlight_line(line, ps).unwrap();
        for (style, text) in ranges {
            let fg_color = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            spans.push(Span::styled(text, Style::default().fg(fg_color)));
        }
        spans.push(Span::raw("\n"));
        highlighted_text.push(ListItem::new(Line::from(spans)));
    }

    List::new(highlighted_text)
}

fn setup_popup<'a, B: Backend>(
    f: &mut Frame<B>,
    title: &'a str,
    height_precentage: u16,
    width_precentage: u16,
) -> (Block<'a>, Rect) {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .set_style(Style::default().gray());

    let y_border_percentage = (100 - height_precentage.min(100)) / 2;
    let y_constraints = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(y_border_percentage),
                Constraint::Percentage(height_precentage),
                Constraint::Percentage(y_border_percentage),
            ]
            .as_ref(),
        )
        .split(f.size());

    let x_border_percentage = (100 - width_precentage.min(100)) / 2;
    let area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(x_border_percentage),
                Constraint::Percentage(width_precentage),
                Constraint::Percentage(x_border_percentage),
            ]
            .as_ref(),
        )
        .split(y_constraints[1])[1];

    (block, area)
}
