use super::lua::*;
use crate::midi::MidiMessageString;
use crate::widgets::StatefulList;
use crossbeam::channel::{Receiver, Sender};
use crossterm::event::KeyCode;
use ratatui::prelude::*;

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

const DOCS: &str = include_str!("../../../api/midimon/docs.lua");
const API: &str = include_str!("../../../api/midimon/api.lua");

pub struct App {
    is_running: bool,
    show_usage: bool,
    show_api: bool,
    show_script: bool,
    show_docs: bool,
    show_alert: bool,

    alert_message: Option<String>,

    is_port_selector_focused: bool,
    script_dir: Option<std::path::PathBuf>,

    messages: crate::widgets::MidiMessageStream,
    port_selector: StatefulList<String>,
    script_selector: StatefulList<String>,

    midi_tx: Sender<MidiMessageString>,
    midi_rx: Receiver<MidiMessageString>,
    midi_in: crate::midi::Input<Sender<MidiMessageString>>,

    selected_port_name: Option<String>,
    selected_script_name: Option<String>,
    loaded_script_content: Option<String>,

    lua: Lua,
    script_tx: Sender<ScriptEvent>,
    script_rx: Receiver<ScriptEvent>,
}

impl Default for App {
    fn default() -> Self {
        let (midi_tx, midi_rx) = crossbeam::channel::bounded(1_000);
        let (script_tx, script_rx) = crossbeam::channel::bounded(1_000);

        Self {
            midi_tx,
            midi_rx,
            script_tx,
            script_rx,
            is_running: true,
            show_api: false,
            show_usage: false,
            show_script: false,
            show_docs: false,
            show_alert: false,
            alert_message: None,
            is_port_selector_focused: true,
            selected_port_name: None,
            port_selector: StatefulList::default(),
            script_selector: StatefulList::default(),
            messages: crate::widgets::MidiMessageStream::default(),
            midi_in: crate::midi::Input::default(),
            script_dir: None,
            selected_script_name: None,
            loaded_script_content: None,
            lua: Lua::new(),
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if self.lua.has_loaded_script() {
            self.lua.on_stop().unwrap();
        }
    }
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let mut app = Self::default();
        app.update_ports()?;
        Ok(app)
    }

    pub fn set_scripts(&mut self, script_dir: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        self.script_dir = Some(script_dir.as_ref().into());

        let entries = std::fs::read_dir(script_dir)?;
        self.script_selector = StatefulList::with_items(
            entries
                .filter_map(|entry| {
                    let path = entry.ok()?.path();
                    if path.is_file() {
                        path.file_name()?.to_str().map(|s| s.to_owned())
                    } else {
                        None
                    }
                })
                .collect(),
        );

        Ok(())
    }

    fn update_ports(&mut self) -> anyhow::Result<()> {
        let input_port_names = self.midi_in.names()?;

        if input_port_names != self.port_selector.items {
            log::info!("MIDI input ports found : {input_port_names:?}");

            if self.lua.has_loaded_script() {
                self.lua.on_discover(&input_port_names)?;
            }

            self.port_selector = StatefulList::with_items(input_port_names);
        }
        Ok(())
    }

    fn connect(&mut self) -> anyhow::Result<()> {
        let Some(index) = self.port_selector.selected() else {
            return Ok(());
        };

        let Some(input_port) = self.port_selector.items.get(index) else {
            anyhow::bail!("Invalid port selection");
        };

        self.midi_in.select(input_port)?;
        self.selected_port_name = self.midi_in.selection();
        self.midi_in.connect(
            {
                let lua = self.lua.clone();
                let device_name = self.selected_port_name.as_ref().unwrap().clone();

                move |timestamp, bytes, sender| {
                    let lua_on_midi = || match lua.on_midi(&device_name, bytes) {
                        Ok(res) => res.unwrap_or(true),
                        Err(e) => {
                            log::error!("{e}");
                            true
                        }
                    };

                    let should_print_message = if lua.has_loaded_script() {
                        lua_on_midi()
                    } else {
                        true
                    };

                    if !should_print_message {
                        return;
                    }

                    if let Some(msg) = MidiMessageString::new(timestamp, bytes) {
                        if let Err(e) = sender.try_send(msg) {
                            log::error!("failed to push midi message : {e}");
                        }
                    }
                }
            },
            self.midi_tx.clone(),
        )?;

        if self.lua.has_loaded_script() {
            if let Some(name) = self.selected_port_name.as_ref() {
                self.lua.on_connect(name)?;
            }
        }

        Ok(())
    }

    fn load_script(&mut self) -> anyhow::Result<()> {
        let Some(ref script_dir) = self.script_dir else {
            anyhow::bail!("Script directory is unspecified");
        };

        let Some(ref script_filename) = self.selected_script_name else {
            anyhow::bail!("No selected script ");
        };

        let script_file = script_dir.join(script_filename);
        if !script_file.exists() || !script_file.is_file() {
            anyhow::bail!("Invalid script path or type");
        }

        if self.lua.has_loaded_script() {
            self.lua.on_stop()?;
        }

        self.loaded_script_content = Some(std::fs::read_to_string(script_file)?);

        self.lua.load(API)?;
        self.lua
            .load(self.loaded_script_content.as_ref().unwrap())?;

        log::info!("script loaded : {script_filename}");

        self.lua
            .setup_api(script_filename, self.script_tx.clone())?;
        self.lua.on_start()?;

        if self.midi_in.is_connected() {
            self.connect()?;
        }

        Ok(())
    }
}

impl crate::app::Base for App {
    fn update(&mut self) -> anyhow::Result<crate::app::Flow> {
        if self.lua.has_loaded_script() {
            self.lua.on_tick()?;
        }

        while let Ok(script_event) = self.script_rx.try_recv() {
            match script_event {
                ScriptEvent::Connect(ref device) => {
                    if let Some(i) = self
                        .port_selector
                        .items
                        .iter()
                        .position(|name| name == device)
                    {
                        self.port_selector.select(i);
                        self.connect()?;
                    }
                }
                ScriptEvent::Alert(msg) => {
                    self.show_alert = true;
                    self.alert_message = Some(msg);
                }
                ScriptEvent::Pause => self.is_running = false,
                ScriptEvent::Resume => self.is_running = true,
                ScriptEvent::Stop => return Ok(crate::app::Flow::Exit),
            }
        }

        self.update_ports()?;

        if !self.is_running {
            return Ok(crate::app::Flow::Loop);
        }

        self.messages.collect(self.midi_rx.try_iter().collect());

        Ok(crate::app::Flow::Continue)
    }

    fn on_keypress(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<crate::app::Flow> {
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
                    return Ok(crate::app::Flow::Exit);
                }
            }
            KeyCode::Char('c') => self.messages.clear(),
            KeyCode::Char(' ') => self.is_running = !self.is_running,
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
                        self.is_running = true;
                        self.messages.clear();
                        self.connect()?;
                    }
                } else if self.script_selector.selected().is_some() {
                    self.script_selector.confirm_selection();
                    self.selected_script_name = self.script_selector.selected_item().cloned();
                    self.load_script()?;
                }
            }
            _ => {}
        }

        Ok(crate::app::Flow::Continue)
    }

    fn render(&mut self, f: &mut Frame<impl Backend>) {
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

        self.port_selector.render_selector(
            f,
            if has_script_dir {
                top_sections[0]
            } else {
                sections[0]
            },
            "˧ ports ꜔",
            self.is_port_selector_focused,
        );

        if has_script_dir {
            self.script_selector.render_selector(
                f,
                top_sections[1],
                "˧ scripts ꜔",
                !self.is_port_selector_focused,
            )
        }

        let selected_port_name = match self.selected_port_name.clone() {
            Some(name) => format!("˧ port : {name} ꜔"),
            None => "".to_owned(),
        };

        let selected_script_name = match self.selected_script_name.clone() {
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
