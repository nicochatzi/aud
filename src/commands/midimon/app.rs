use super::{lua::*, ui::*};
use crate::widgets::ListView;
use crate::{lua::traits::api::*, midi::MidiMessageString};
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
    port_selector: ListView,
    script_selector: ListView,

    port_names: Vec<String>,
    selected_port_name: Option<String>,

    script_names: Vec<String>,
    selected_script_name: Option<String>,

    host_tx: Sender<HostEvent>,
    script_rx: Receiver<ScriptEvent>,
    lua_handle: LuaRuntimeHandle,

    midi_in: crate::midi::Input<Sender<HostEvent>>,

    loaded_script_content: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        let (host_tx, host_rx) = crossbeam::channel::bounded::<HostEvent>(1_000);
        let (script_tx, script_rx) = crossbeam::channel::bounded::<ScriptEvent>(1_000);

        Self {
            host_tx,
            script_rx,
            lua_handle: LuaRuntime::start(host_rx, script_tx.clone()),
            is_running: true,
            show_api: false,
            show_usage: false,
            show_script: false,
            show_docs: false,
            show_alert: false,
            alert_message: None,
            is_port_selector_focused: true,
            selected_port_name: None,
            port_selector: ListView::default(),
            port_names: vec![],
            script_selector: ListView::default(),
            script_names: vec![],
            messages: crate::widgets::MidiMessageStream::default(),
            midi_in: crate::midi::Input::default(),
            script_dir: None,
            selected_script_name: None,
            loaded_script_content: None,
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let Some(handle) = self.lua_handle.take_handle() else {
            return;
        };

        if let Err(e) = self.host_tx.try_send(HostEvent::Terminate) {
            log::error!("Failed to send termination message to Lua runtime : {e}");
            return;
        };

        if handle.join().is_err() {
            log::error!("Failed to join on Lua runtime thread handle");
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

    fn update_ports(&mut self) -> anyhow::Result<()> {
        let input_port_names = self.midi_in.names()?;

        if input_port_names != self.port_names {
            log::info!("MIDI input ports found : {input_port_names:?}");

            let event = HostEvent::Discover(input_port_names.clone());
            if let Err(e) = self.host_tx.try_send(event) {
                log::error!("Failed to send device discovery event to runtime : {e}");
            }

            self.port_names = input_port_names;
            self.port_selector = ListView::with_len(self.port_names.len());
        }
        Ok(())
    }

    fn connect(&mut self) -> anyhow::Result<()> {
        let Some(index) = self.port_selector.selected() else {
            return Ok(());
        };

        let Some(input_port) = self.port_names.get(index) else {
            anyhow::bail!("Invalid port selection");
        };

        self.midi_in.select(input_port)?;
        self.selected_port_name = self.midi_in.selection();
        self.midi_in.connect(
            {
                move |timestamp, bytes, sender| {
                    let midi = MidiData {
                        timestamp,
                        bytes: bytes.into(),
                    };
                    if let Err(e) = sender.try_send(HostEvent::Midi(midi)) {
                        log::error!("Failed to push midi message event to runtime : {e}");
                    }
                }
            },
            self.host_tx.clone(),
        )?;

        if let Some(name) = self.selected_port_name.as_ref() {
            if let Err(e) = self.host_tx.try_send(HostEvent::Connect(name.to_string())) {
                log::error!("Failed to send device connected event to runtime : {e}");
            }
        }

        Ok(())
    }

    fn load_script(&mut self) -> anyhow::Result<bool> {
        let Some(index) = self.script_selector.selected() else {
            return Ok(false);
        };

        let Some(script_name) = self.script_names.get(index) else {
            anyhow::bail!("Invalid port selection");
        };

        self.selected_script_name = Some(script_name.into());

        let Some(ref script_dir) = self.script_dir else {
            anyhow::bail!("Script directory is unspecified");
        };

        let Some(ref script_filename) = self.selected_script_name else {
            anyhow::bail!("No selected script");
        };

        let script_file = script_dir.join(script_filename);
        if !script_file.exists() || !script_file.is_file() {
            anyhow::bail!("Invalid script path or type");
        }

        if let Err(e) = self.host_tx.try_send(HostEvent::Stop) {
            log::error!("failed to send stop event : {e}");
        }

        let chunk = std::fs::read_to_string(script_file)?;
        self.loaded_script_content = Some(chunk.clone());

        let event = HostEvent::LoadScript {
            name: script_filename.to_owned(),
            chunk,
        };

        if let Err(e) = self.host_tx.try_send(event) {
            log::error!("failed to send load script event : {e}");
        }

        if self.midi_in.is_connected() {
            self.connect()?;
        }

        Ok(true)
    }

    fn handle_lua_connect_request(&mut self, request: ConnectionApiEvent) -> anyhow::Result<()> {
        let ConnectionApiEvent { ref device } = request;

        if let Some(i) = self.port_names.iter().position(|name| name == device) {
            self.port_selector.select(i);
            self.connect()?;
        }

        Ok(())
    }

    fn handle_lua_control_request(&mut self, request: ControlFlowApiEvent) -> bool {
        match request {
            ControlFlowApiEvent::Pause => self.is_running = false,
            ControlFlowApiEvent::Resume => self.is_running = true,
            ControlFlowApiEvent::Stop => return false,
        }

        true
    }

    fn handle_lua_log_request(&mut self, request: LogApiEvent) {
        match request {
            LogApiEvent::Log(msg) => log::info!("{msg}"),
            LogApiEvent::Alert(msg) => {
                self.show_alert = true;
                self.alert_message = Some(msg);
            }
        }
    }
}

impl crate::app::Base for App {
    fn update(&mut self) -> anyhow::Result<crate::app::Flow> {
        while let Ok(script_event) = self.script_rx.try_recv() {
            match script_event {
                ScriptEvent::Connect(request) => self.handle_lua_connect_request(request)?,
                ScriptEvent::Control(request) => {
                    if !self.handle_lua_control_request(request) {
                        return Ok(crate::app::Flow::Exit);
                    }
                }
                ScriptEvent::Log(request) => self.handle_lua_log_request(request),
                ScriptEvent::Midi(midi) => {
                    if let Some(midi) = MidiMessageString::new(midi.timestamp, &midi.bytes) {
                        self.messages.push(midi)
                    }
                }
            }
        }

        self.update_ports()?;

        if !self.is_running {
            return Ok(crate::app::Flow::Loop);
        }

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

        let port_selector_section = if has_script_dir {
            top_sections[0]
        } else {
            sections[0]
        };

        self.port_selector.render_selector(
            f,
            port_selector_section,
            "˧ ports ꜔",
            &self.port_names,
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
