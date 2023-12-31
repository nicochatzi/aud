mod ui;

use crate::ui::widgets::midi::MidiMessageString;
use aud::{
    controllers::audio_midi::{AppEvent, AudioMidiController},
    lua::imported,
    midi::HostedMidiReceiver,
};
use ratatui::prelude::*;

struct TerminalApp {
    ui: ui::Ui,
    app: AudioMidiController,
}

impl Default for TerminalApp {
    fn default() -> Self {
        let midi_in = Box::<HostedMidiReceiver>::default();
        let app = AudioMidiController::with_midi(midi_in, imported::midimon::API);
        let mut ui = ui::Ui::default();
        ui.update_port_names(app.midi().port_names());
        Self { ui, app }
    }
}

impl crate::app::Base for TerminalApp {
    fn update(&mut self) -> anyhow::Result<crate::app::Flow> {
        self.app.midi_mut().update();
        self.app.process_engine_events()?;

        if self.app.process_script_events()? == AppEvent::Stopping {
            return Ok(crate::app::Flow::Exit);
        }

        let mut messages: Vec<_> = self
            .app
            .midi_mut()
            .take_messages()
            .iter()
            .filter_map(|midi| MidiMessageString::new(midi.timestamp, &midi.bytes))
            .collect();

        self.ui.append_messages(&mut messages);

        if self.app.process_file_events()? == AppEvent::ScriptLoaded {
            self.ui.clear_script_cache();
        }

        Ok(crate::app::Flow::Continue)
    }

    fn on_keypress(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<crate::app::Flow> {
        match self.ui.handle_keypress(key)? {
            ui::UiEvent::Continue => (),
            ui::UiEvent::Exit => return Ok(crate::app::Flow::Exit),
            ui::UiEvent::ToggleRunningState => {
                let run = !self.app.midi().is_running();
                self.app.midi_mut().set_running(run)
            }
            ui::UiEvent::ClearMessages => self.app.midi_mut().clear_messages(),
            ui::UiEvent::Connect(port_index) => {
                self.app.midi_mut().connect_to_input_by_index(port_index)?;
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

    fn render(&mut self, f: &mut Frame) {
        if let Some(alert) = self.app.take_alert() {
            self.ui.show_alert_message(&alert);
        }

        self.ui.render(f, &self.app);
    }
}

#[derive(Debug, clap::Parser)]
pub struct Options {
    /// Path to log file to write to. Defaults
    /// to system log file at ~/.aud/log/auscope.log
    #[arg(long)]
    log: Option<std::path::PathBuf>,

    /// Frames per second
    #[arg(long, default_value_t = 30.)]
    fps: f32,

    /// Path to scripts to view or default script to load
    #[arg(long)]
    script: Option<std::path::PathBuf>,
}

pub fn run(
    terminal: &mut Terminal<impl Backend>,
    opts: Options,
    common_opts: crate::CommonOptions,
) -> anyhow::Result<()> {
    if let Some(log_file) = opts.log.or_else(|| crate::locations::log_file("midimon")) {
        crate::logger::start("midimon", log_file, common_opts.verbose)?;
    }

    let mut app = TerminalApp::default();

    let scripts = opts
        .script
        .or_else(|| crate::locations::lua::examples_for("midimon"));

    if let Some(script) = scripts {
        log::info!("{:#?}", script.canonicalize()?);
        app.ui.update_script_dir(script)?;
    }

    crate::app::run(terminal, &mut app, opts.fps.max(1.))
}
