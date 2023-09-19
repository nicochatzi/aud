mod lua;
mod ui;

pub mod app;

use ratatui::prelude::*;
use std::path::PathBuf;

struct TerminalApp {
    ui: ui::Ui,
    app: app::App,
}

impl Default for TerminalApp {
    fn default() -> Self {
        let app = app::App::default();
        let mut ui = ui::Ui::default();
        ui.update_port_names(app.ports());
        Self { ui, app }
    }
}

impl crate::app::Base for TerminalApp {
    fn update(&mut self) -> anyhow::Result<crate::app::Flow> {
        if !self.app.process_script_events()? {
            return Ok(crate::app::Flow::Exit);
        }

        let has_file_changed = self.app.process_file_events()?;
        if has_file_changed {
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
                let Some(script_name) = &self.ui.scripts().get(script_index) else {
                    return Ok(crate::app::Flow::Continue);
                };

                let script_path = self.ui.script_dir().unwrap().join(script_name);
                self.app.load_script(script_path)?;
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
    match log.or(crate::files::log()) {
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
