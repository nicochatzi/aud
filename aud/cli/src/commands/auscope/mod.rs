mod ui;

use aud::streams::audio::stream::HostedAudioReceiver;
use ratatui::prelude::*;
use std::path::PathBuf;

type AuscopeApp = aud::apps::auscope::app::App<HostedAudioReceiver>;

struct TerminalApp {
    app: AuscopeApp,
    ui: ui::Ui,
}

impl Default for TerminalApp {
    fn default() -> Self {
        let mut app = AuscopeApp::with_audio_receiver(HostedAudioReceiver::default());
        app.update_device_list().unwrap();

        let mut ui = ui::Ui::default();
        ui.update_device_names(app.device_names());

        Self { app, ui }
    }
}

impl crate::app::Base for TerminalApp {
    fn update(&mut self) -> anyhow::Result<crate::app::Flow> {
        self.app.fetch_audio();
        Ok(crate::app::Flow::Continue)
    }

    fn on_keypress(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<crate::app::Flow> {
        match self.ui.on_keypress(key) {
            ui::UiEvent::Continue => Ok(crate::app::Flow::Continue),
            ui::UiEvent::Exit => Ok(crate::app::Flow::Exit),
            ui::UiEvent::Select { id, index } => match id {
                ui::Selector::Device => {
                    for buf in self.app.audio_mut() {
                        buf.clear();
                    }
                    self.app.connect_to_audio_input(index)?;
                    Ok(crate::app::Flow::Continue)
                }
                ui::Selector::Script => Ok(crate::app::Flow::Continue),
            },
        }
    }

    fn render(&mut self, f: &mut Frame<impl Backend>) {
        self.ui.render(f, &mut self.app);
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
        Some(log_file) => crate::logger::start("audscope", log_file),
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
