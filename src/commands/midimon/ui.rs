use super::{
    lua::{API, DOCS},
    state::State,
};
use crate::widgets::ListView;
use crossterm::event::KeyCode;
use ratatui::prelude::*;
use std::path::Path;

const USAGE: &str = r#"
         ? : display help
         a : display API
         s : display script
         d : display docs
   <SPACE> : pause / resume
   <UP>, k : scroll up
 <DOWN>, j : scroll down
 <LEFT>, h : cycle panes left
<RIGHT>, l : cycle panes right
 <DOWN>, j : scroll down
     Enter : confirm selection
  <ESC>, q : quit or hide popup
     <C-c> : force quit
"#;

pub struct Ui {
    show_usage: bool,
    show_api: bool,
    show_script: bool,
    show_docs: bool,
    show_alert: bool,

    alert_message: Option<String>,

    is_port_selector_focused: bool,

    script_dir: Option<std::path::PathBuf>,
    script_names: Vec<String>,

    messages: crate::widgets::MidiMessageStream,
    port_selector: ListView,
    script_selector: ListView,

    loaded_script_content: Option<String>,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            show_api: false,
            show_usage: false,
            show_script: false,
            show_docs: false,
            show_alert: false,
            alert_message: None,
            is_port_selector_focused: true,
            port_selector: ListView::default(),
            script_selector: ListView::default(),
            messages: crate::widgets::MidiMessageStream::default(),
            script_dir: None,
            script_names: vec![],
            loaded_script_content: None,
        }
    }
}

pub enum UiEvent {
    Continue,
    ToggleRunningState,
    Connect(usize),
    LoadScript(usize),
    Exit,
}

impl Ui {
    pub fn set_script_dir(&mut self, script_dir: impl AsRef<Path>) -> anyhow::Result<()> {
        let script_dir = script_dir.as_ref();
        self.script_names = std::fs::read_dir(script_dir)?
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_file() {
                    path.file_name()?.to_str().map(|s| s.to_owned())
                } else {
                    None
                }
            })
            .collect();

        self.script_dir = Some(script_dir.into());
        self.script_selector = ListView::with_len(self.script_names.len());
        Ok(())
    }

    pub fn handle_keypress(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<UiEvent> {
        match key.code {
            KeyCode::Char('?') => self.show_usage = !self.show_usage,
            KeyCode::Char('a') => {
                self.show_api = !self.show_api;
                if self.show_api {
                    self.show_docs = false;
                    self.show_script = false;
                }
            }
            KeyCode::Char('s') => {
                self.show_script = !self.show_script;
                if self.show_script {
                    self.show_docs = false;
                    self.show_api = false;
                }
            }
            KeyCode::Char('d') => {
                self.show_docs = !self.show_docs;
                if self.show_docs {
                    self.show_script = false;
                    self.show_api = false;
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.show_usage {
                    self.show_usage = false
                } else if self.show_script {
                    self.show_script = false
                } else if self.show_docs {
                    self.show_docs = false
                } else if self.show_api {
                    self.show_api = false
                } else if self.show_alert {
                    self.show_alert = false
                } else {
                    return Ok(UiEvent::Exit);
                }
            }
            KeyCode::Char('c') => self.messages.clear(),
            KeyCode::Char(' ') => return Ok(UiEvent::ToggleRunningState),
            KeyCode::Left | KeyCode::Char('h') => {
                self.is_port_selector_focused = !self.is_port_selector_focused
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.is_port_selector_focused = !self.is_port_selector_focused
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.is_port_selector_focused {
                    self.port_selector.next()
                } else {
                    self.script_selector.next()
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.is_port_selector_focused {
                    self.port_selector.previous()
                } else {
                    self.script_selector.previous()
                }
            }
            KeyCode::Enter => {
                if self.is_port_selector_focused {
                    if self.port_selector.selected().is_some() {
                        self.port_selector.confirm_selection();
                        return Ok(UiEvent::Connect(self.port_selector.selected().unwrap()));
                    }
                } else if self.script_selector.selected().is_some() {
                    self.script_selector.confirm_selection();
                    return Ok(UiEvent::LoadScript(
                        self.script_selector.selected().unwrap(),
                    ));
                }
            }
            _ => {}
        }

        Ok(UiEvent::Continue)
    }

    pub fn render(&mut self, f: &mut Frame<impl Backend>, state: &State) {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(3), Constraint::Percentage(80)].as_ref())
            .split(f.size());

        let top_sections = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(sections[0]);

        let has_script_dir = self.script_dir.as_ref().is_some_and(|dir| dir.is_dir());

        let port_selector_section = if has_script_dir {
            top_sections[0]
        } else {
            sections[0]
        };

        self.port_selector.render_selector(
            f,
            port_selector_section,
            "˧ ports ꜔",
            state.ports(),
            self.is_port_selector_focused,
        );

        if has_script_dir {
            self.script_selector.render_selector(
                f,
                top_sections[1],
                "˧ scripts ꜔",
                &self.script_names,
                !self.is_port_selector_focused,
            )
        }

        let selected_port_name = match state.selected_port() {
            Some(name) => format!("˧ port : {name} ꜔"),
            None => "".to_owned(),
        };

        let selected_script_name = match state.selected_script() {
            Some(name) => format!("˧ script : {name} ꜔"),
            None => "".to_owned(),
        };

        f.render_widget(
            self.messages
                .make_list_view(&format!("{selected_port_name}───{selected_script_name}")),
            sections[1],
        );

        if self.show_api {
            crate::widgets::text::render_code_popup(f, "˧ API ꜔", API);
        }

        if self.show_docs {
            crate::widgets::text::render_code_popup(f, "˧ docs ꜔", DOCS);
        }

        if self.show_script {
            let text = match self.loaded_script_content.as_ref() {
                Some(code) => code,
                None => "No script loaded",
            };

            crate::widgets::text::render_code_popup(f, "˧ loaded script ꜔", text);
        }

        if self.show_usage {
            crate::widgets::text::render_usage_popup(f, USAGE);
        }

        if self.show_alert {
            if let Some(ref msg) = self.alert_message {
                crate::widgets::text::render_alert_popup(f, msg);
            }
        }
    }
}
