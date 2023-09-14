use ratatui::prelude::*;

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
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Min(32), Constraint::Percentage(90)].as_ref())
        .split(f.size());

    app.device_names
        .render_selector(f, sections[0], "˧ devices ꜔");

    let selected_device_name = match app.selection.clone() {
        Some(name) => format!("˧ {name} ꜔"),
        None => "".to_owned(),
    };

    crate::widgets::scope::render(f, sections[1], &selected_device_name, &mut app.audio_buffer);

    if app.show_usage {
        crate::widgets::usage::render(f, USAGE);
    }
}
