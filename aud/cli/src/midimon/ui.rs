use crate::ui::{components, widgets};
use aud::{apps::midimon::App, files};
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
     Enter : confirm selection
  <ESC>, q : quit or hide popup
     <C-c> : force quit
"#;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Popup {
    Usage,
    Api,
    Docs,
    Script,
    Alert,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Selector {
    Script,
    Port,
}

pub struct Ui {
    popups: components::Popups<Popup>,
    selectors: components::Selectors<Selector>,
    alert_message: Option<String>,
    script_dir: Option<std::path::PathBuf>,
    script_names: Vec<String>,
    cached_script: Option<String>,
    messages: Vec<widgets::midi::MidiMessageString>,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            popups: components::Popups::new(&[
                (Popup::Usage, components::PopupKind::Text),
                (Popup::Api, components::PopupKind::Code),
                (Popup::Docs, components::PopupKind::Code),
                (Popup::Script, components::PopupKind::Code),
                (Popup::Alert, components::PopupKind::Text),
            ]),
            selectors: components::Selectors::new(&[Selector::Script, Selector::Port]),
            alert_message: None,
            script_dir: None,
            script_names: vec![],
            cached_script: None,
            messages: vec![],
        }
    }
}

pub enum UiEvent {
    Continue,
    ToggleRunningState,
    ClearMessages,
    Connect(usize),
    LoadScript(usize),
    Exit,
}

impl Ui {
    pub fn scripts(&self) -> &[String] {
        self.script_names.as_slice()
    }

    pub fn script_dir(&self) -> Option<&std::path::PathBuf> {
        self.script_dir.as_ref()
    }

    pub fn clear_script_cache(&mut self) {
        self.cached_script = None;
    }

    pub fn append_messages(&mut self, messages: &mut Vec<widgets::midi::MidiMessageString>) {
        self.messages.append(messages);
    }

    pub fn update_port_names(&mut self, port_names: &[impl AsRef<str>]) {
        if let Some(sel) = self.selectors.get_mut(Selector::Port) {
            *sel = components::Selector::with_len(port_names.len());
        }
    }

    pub fn show_alert_message(&mut self, alert_message: &str) {
        self.popups.show(Popup::Alert);
        self.alert_message = Some(alert_message.into());
    }

    pub fn update_script_dir(&mut self, dir: impl AsRef<Path>) -> anyhow::Result<()> {
        let dir = dir.as_ref();
        self.script_names = files::list_with_extension(dir, "lua")?;
        self.script_dir = Some(dir.into());
        if let Some(sel) = self.selectors.get_mut(Selector::Script) {
            *sel = components::Selector::with_len(self.script_names.len());
        }
        Ok(())
    }

    pub fn handle_keypress(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<UiEvent> {
        match key.code {
            KeyCode::Char('?') => self.popups.toggle_visible(Popup::Usage),
            KeyCode::Char('a') => self.popups.toggle_visible(Popup::Api),
            KeyCode::Char('s') => self.popups.toggle_visible(Popup::Script),
            KeyCode::Char('d') => self.popups.toggle_visible(Popup::Docs),
            KeyCode::Char('q') | KeyCode::Esc => {
                if !self.popups.any_visible() {
                    return Ok(UiEvent::Exit);
                }

                self.popups.hide()
            }
            KeyCode::Char('c') => return Ok(UiEvent::ClearMessages),
            KeyCode::Char(' ') => return Ok(UiEvent::ToggleRunningState),
            KeyCode::Left | KeyCode::Char('h') => self.selectors.previous_selector(),
            KeyCode::Right | KeyCode::Char('l') => self.selectors.next_selector(),
            KeyCode::Down | KeyCode::Char('j') => self.selectors.next_item(),
            KeyCode::Up | KeyCode::Char('k') => self.selectors.previous_item(),
            KeyCode::Enter => {
                if let Some(selection) = self.selectors.select() {
                    match selection.selector {
                        Selector::Port => return Ok(UiEvent::Connect(selection.index)),
                        Selector::Script => {
                            self.cached_script = None;
                            return Ok(UiEvent::LoadScript(selection.index));
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(UiEvent::Continue)
    }

    pub fn render(&mut self, f: &mut Frame, app: &App) {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(3), Constraint::Percentage(80)].as_ref())
            .split(f.size());

        let top_sections = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(sections[0]);

        let has_script_dir = self.script_dir.as_ref().is_some_and(|dir| dir.is_dir());
        let port_selector_section = if has_script_dir {
            top_sections[0]
        } else {
            sections[0]
        };

        self.selectors.render(
            f,
            port_selector_section,
            Selector::Port,
            crate::title!("ports"),
            app.ports(),
        );

        if has_script_dir {
            self.selectors.render(
                f,
                top_sections[1],
                Selector::Script,
                &crate::title!("{}", self.script_dir.as_ref().unwrap().to_string_lossy()),
                &self.script_names,
            )
        }

        let selected_port_name = match app.selected_port() {
            Some(name) => crate::title!("port : {}", name),
            None => "".to_owned(),
        };

        let selected_script_name = match app.selected_script() {
            Some(name) => crate::title!("script : {}", name),
            None => "".to_owned(),
        };

        let running_state = if app.running() {
            crate::title!("active")
        } else {
            crate::title!("paused")
        };

        widgets::midi::render_messages(
            f,
            &format!("{running_state}─{selected_port_name}─{selected_script_name}"),
            &self.messages,
            sections[1],
        );

        self.popups.render(
            f,
            Popup::Api,
            crate::title!("api"),
            aud::lua::imported::midimon::API,
        );

        self.popups.render(
            f,
            Popup::Docs,
            crate::title!("docs"),
            aud::lua::imported::midimon::DOCS,
        );

        self.popups
            .render(f, Popup::Usage, crate::title!("usage"), USAGE);

        self.popups.render(
            f,
            Popup::Alert,
            crate::title!("alert!"),
            self.alert_message.as_ref().unwrap_or(&"".to_owned()),
        );

        if !self.popups.is_visible(Popup::Script) {
            self.popups
                .render(f, Popup::Script, crate::title!(""), "No script loaded");
            return;
        }

        if self.cached_script.is_none() {
            self.cached_script = Some(
                app.loaded_script_path()
                    .and_then(|path| std::fs::read_to_string(path).ok())
                    .unwrap_or_else(|| "No script loaded".to_owned()),
            );
        }

        self.popups.render(
            f,
            Popup::Script,
            &selected_script_name,
            self.cached_script.as_ref().unwrap(),
        );
    }
}
