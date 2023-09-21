mod ui;

use aud::audio::HostedAudioProducer;
use ratatui::prelude::*;

type AuscopeApp = aud::apps::auscope::app::App<HostedAudioProducer>;

struct TerminalApp {
    app: AuscopeApp,
    ui: ui::Ui,
}

impl Default for TerminalApp {
    fn default() -> Self {
        let app = AuscopeApp::with_audio_receiver(HostedAudioProducer::default());
        let mut ui = ui::Ui::default();
        ui.update_device_names(app.devices());

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
            ui::UiEvent::Select { id, .. } => match id {
                ui::Selector::Device => {
                    for buf in self.app.audio_mut() {
                        buf.clear();
                    }
                    // self.app.connect_to_audio_input(index)?;
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

    /// Path to scripts to view or default script to load
    #[arg(long)]
    script: Option<std::path::PathBuf>,
}

pub fn run(terminal: &mut Terminal<impl Backend>, opts: Options) -> anyhow::Result<()> {
    if let Some(log_file) = opts.log.or(crate::locations::log_file()) {
        crate::logger::start("audscope", log_file)?;
    }

    let mut app = TerminalApp::default();

    let scripts = opts
        .script
        .or(crate::locations::lua::examples_for("auscope"));

    if let Some(script) = scripts {
        log::info!("{:#?}", script.canonicalize()?);
        app.ui.update_script_dir(script)?;
    }

    crate::app::run(terminal, &mut app, opts.fps.max(1.))
}
