use crate::ui::{components, widgets};
use aud::{
    apps::auscope::lua::{API, DOCS},
    files,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;

const USAGE: &str = r#"
         ? : display help
   <UP>, k : scroll up
 <DOWN>, j : scroll down
     Enter : confirm selection
  <ESC>, q : quit or hide help
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
pub enum Selector {
    Device,
    Script,
}

pub enum UiEvent<Id> {
    Continue,
    Exit,
    Select { id: Id, index: usize },
}

pub struct Ui {
    popups: components::Popups<Popup>,
    selectors: components::Selectors<Selector>,
    script_dir: Option<std::path::PathBuf>,
    script_names: Vec<String>,
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
            selectors: components::Selectors::new(&[Selector::Device, Selector::Script]),
            script_dir: None,
            script_names: vec![],
        }
    }
}

impl Ui {
    pub fn update_script_dir(&mut self, dir: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let dir = dir.as_ref();
        self.script_names = files::list_with_extension(dir, "lua")?;
        self.script_dir = Some(dir.into());
        if let Some(sel) = self.selectors.get_mut(Selector::Script) {
            *sel = components::Selector::with_len(self.script_names.len());
        }
        Ok(())
    }

    pub fn update_device_names(&mut self, names: &[impl AsRef<str>]) {
        if let Some(devices) = self.selectors.get_mut(Selector::Device) {
            *devices = components::Selector::with_len(names.len());
        };
    }

    pub fn on_keypress(&mut self, key: KeyEvent) -> UiEvent<Selector> {
        match key.code {
            KeyCode::Char('?') => self.popups.toggle_visible(Popup::Usage),
            KeyCode::Char('q') | KeyCode::Esc => {
                if !self.popups.any_visible() {
                    return UiEvent::Exit;
                }
                self.popups.hide()
            }
            KeyCode::Down | KeyCode::Char('j') => self.selectors.next_item(),
            KeyCode::Up | KeyCode::Char('k') => self.selectors.previous_item(),
            KeyCode::Left | KeyCode::Char('h') => self.selectors.previous_selector(),
            KeyCode::Right | KeyCode::Char('l') => self.selectors.next_selector(),
            KeyCode::Enter => {
                if let Some(selection) = self.selectors.select() {
                    return UiEvent::Select {
                        id: selection.selector,
                        index: selection.index,
                    };
                }
            }
            _ => {}
        }

        UiEvent::Continue
    }

    pub fn render(&mut self, f: &mut Frame<impl Backend>, app: &mut super::AuscopeApp) {
        let sections = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Min(32), Constraint::Percentage(90)].as_ref())
            .split(f.size());

        let left_sections = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(sections[0]);

        let has_script_dir = self.script_dir.as_ref().is_some_and(|dir| dir.is_dir());
        let device_selector_section = if has_script_dir {
            left_sections[0]
        } else {
            sections[0]
        };

        self.selectors.render(
            f,
            device_selector_section,
            Selector::Device,
            crate::title!("devices"),
            app.device_names(),
        );

        if has_script_dir {
            self.selectors.render(
                f,
                left_sections[1],
                Selector::Script,
                &crate::title!("{}", self.script_dir.as_ref().unwrap().to_string_lossy()),
                self.script_names.as_slice(),
            );
        }

        self.popups.render(f, Popup::Api, crate::title!("api"), API);
        self.popups
            .render(f, Popup::Docs, crate::title!("docs"), DOCS);
        self.popups
            .render(f, Popup::Usage, crate::title!("usage"), USAGE);

        // self.popups.render(f, Popup::Script, );
        // self.popups.render(f, Popup::Aler, );

        let selected_device_name = match self.selectors.get(Selector::Device) {
            Some(s) => s
                .selected()
                .and_then(|index| app.device_names().get(index))
                .map(|name| format!("˧ {name} ꜔"))
                .unwrap_or_else(|| "".to_owned()),
            None => "".to_owned(),
        };

        widgets::scope::render(f, sections[1], &selected_device_name, app.audio_mut());
    }
}
