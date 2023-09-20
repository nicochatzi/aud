mod ui;

use crate::ui::widgets::midi::MidiMessageString;
use ratatui::prelude::*;
use std::path::PathBuf;

type MidimonApp = aud::apps::midimon::app::App;
type MidimonEvent = aud::apps::midimon::app::AppEvent;

struct TerminalApp {
    ui: ui::Ui,
    app: MidimonApp,
}

impl Default for TerminalApp {
    fn default() -> Self {
        let app = MidimonApp::default();
        let mut ui = ui::Ui::default();
        ui.update_port_names(app.ports());
        Self { ui, app }
    }
}

impl crate::app::Base for TerminalApp {
    fn update(&mut self) -> anyhow::Result<crate::app::Flow> {
        if matches!(self.app.process_script_events()?, MidimonEvent::Stopping) {
            return Ok(crate::app::Flow::Exit);
        }

        let mut messages: Vec<_> = self
            .app
            .take_messages()
            .iter()
            .filter_map(|midi| MidiMessageString::new(midi.timestamp, &midi.bytes))
            .collect();

        self.ui.append_messages(&mut messages);

        let was_script_loaded = self
            .app
            .process_file_events()?
            .filter(|e| matches!(e, MidimonEvent::ScriptLoaded));

        if was_script_loaded.is_some() {
            self.ui.clear_script_cache();
        }

        Ok(crate::app::Flow::Continue)
    }

    fn on_keypress(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<crate::app::Flow> {
        match self.ui.handle_keypress(key)? {
            ui::UiEvent::Continue => (),
            ui::UiEvent::Exit => return Ok(crate::app::Flow::Exit),
            ui::UiEvent::ToggleRunningState => self.app.set_running(!self.app.running()),
            ui::UiEvent::ClearMessages => self.app.clear_messages(),
            ui::UiEvent::Connect(port_index) => {
                self.app.connect_to_midi_input_by_index(port_index)?;
            }
            ui::UiEvent::LoadScript(script_index) => {
                if let Some(script_name) = &self.ui.scripts().get(script_index) {
                    let script = self.ui.script_dir().unwrap().join(script_name);
                    self.app.load_script(script)?;
                };
            }
        }

        Ok(crate::app::Flow::Continue)
    }

    fn render(&mut self, f: &mut Frame<impl Backend>) {
        if let Some(alert) = self.app.take_alert() {
            self.ui.show_alert_message(&alert);
        }

        self.ui.render(f, &self.app);
    }
}

#[derive(Debug, clap::Parser)]
pub struct Options {
    /// Path to log file to write to
    #[arg(long)]
    log: Option<std::path::PathBuf>,

    /// Frames per second
    #[arg(long, default_value_t = 30.)]
    fps: f32,

    /// Path to script to load or directory to find scripts
    #[arg(long)]
    script: Option<std::path::PathBuf>,
}

fn start_logger(log: Option<PathBuf>) -> anyhow::Result<()> {
    match log.or(crate::locations::log_file()) {
        Some(log_file) => crate::logger::start("midimon", log_file),
        None => Ok(()),
    }
}

pub fn run_headless(opts: Options) -> anyhow::Result<()> {
    start_logger(opts.log)?;
    Ok(())
}

pub fn run_with_tui(terminal: &mut Terminal<impl Backend>, opts: Options) -> anyhow::Result<()> {
    start_logger(opts.log)?;

    let mut app = TerminalApp::default();

    if let Some(script) = opts.script {
        log::info!("{:#?}", script.canonicalize()?);
        app.ui.update_script_dir(script)?;
    }

    crate::app::run(terminal, &mut app, opts.fps.max(1.))
}
