mod ui;

use std::net::UdpSocket;

use aud::{apps::auscope::*, audio::*, comms::Sockets};
use ratatui::prelude::*;

struct TerminalApp {
    app: App,
    ui: ui::Ui,
}

impl TerminalApp {
    fn new(audio_provider: Box<dyn AudioProvider>) -> Self {
        let app = App::new(audio_provider);
        let mut ui = ui::Ui::default();
        ui.update_device_names(app.devices());
        Self { app, ui }
    }

    fn try_connect_to_audio_input(&mut self, index: usize) -> anyhow::Result<()> {
        let Some(device) = self.app.devices().get(index) else {
            log::warn!(
                "Invalid device index selection {index}, with {} devices",
                self.app.devices().len()
            );
            return Ok(());
        };

        self.app
            .connect_to_audio_input(&device.clone(), AudioChannelSelection::Mono(0))
    }
}

impl crate::app::Base for TerminalApp {
    fn update(&mut self) -> anyhow::Result<crate::app::Flow> {
        self.app.fetch_audio()?;
        Ok(crate::app::Flow::Continue)
    }

    fn on_keypress(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<crate::app::Flow> {
        match self.ui.on_keypress(key) {
            ui::UiEvent::Continue => Ok(crate::app::Flow::Continue),
            ui::UiEvent::Exit => Ok(crate::app::Flow::Exit),
            ui::UiEvent::Select { id, index } => match id {
                ui::Selector::Device => {
                    self.try_connect_to_audio_input(index)?;
                    Ok(crate::app::Flow::Continue)
                }
                ui::Selector::Script => Ok(crate::app::Flow::Continue),
            },
        }
    }

    fn render(&mut self, f: &mut Frame) {
        self.ui.render(f, &mut self.app);
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

    /// Path to scripts to view or default script to run
    #[arg(long)]
    script: Option<std::path::PathBuf>,

    /// Flag to activate remote audio reception.
    /// By default the app uses the system audio device
    #[arg(long, default_value_t = false)]
    remote: bool,

    /// Fetch audio from this remote address
    #[arg(long, default_value = "127.0.0.1")]
    address: String,

    /// Fetch audio using these ports
    #[arg(long, default_value = "8080,8081")]
    ports: String,
}

fn create_remote_audio_provider(address: String, ports: String) -> Box<dyn AudioProvider> {
    let (in_port, out_port) = ports.split_at(
        ports
            .find(|c| c == ',')
            .expect("Invalid ports syntax. Use comma seperate"),
    );

    let sockets = Sockets {
        socket: UdpSocket::bind(format!("{address}:{out_port}")).unwrap(),
        target: format!("{address}:{in_port}").parse().unwrap(),
    };

    let provider =
        RemoteAudioProvider::new(sockets).expect("failed to create remote audio receiver");

    Box::new(provider)
}

pub fn run(
    terminal: &mut Terminal<impl Backend>,
    opts: Options,
    common_opts: crate::CommonOptions,
) -> anyhow::Result<()> {
    if let Some(log_file) = opts.log.or_else(|| crate::locations::log_file("auscope")) {
        crate::logger::start("auscope", log_file, common_opts.verbose)?;
    }

    let audio_provider = if opts.remote {
        create_remote_audio_provider(opts.address, opts.ports)
    } else {
        Box::<HostAudioInput>::default()
    };

    let mut app = TerminalApp::new(audio_provider);

    let scripts = opts
        .script
        .or(crate::locations::lua::examples_for("auscope"));

    if let Some(script) = scripts {
        log::info!("{:#?}", script.canonicalize()?);
        app.ui.update_script_dir(script)?;
    }

    crate::app::run(terminal, &mut app, opts.fps.max(1.))
}
