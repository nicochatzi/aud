mod app;
mod ui;

use ratatui::prelude::*;

#[derive(Debug, clap::Parser)]
pub struct Options {
    /// Path to log file to write to
    #[arg(long)]
    log: Option<std::path::PathBuf>,

    /// Frames per second
    #[arg(long, default_value_t = 30.)]
    fps: f32,
}

pub fn run(terminal: &mut Terminal<impl Backend>, opts: Options) -> anyhow::Result<()> {
    if let Some(log_file) = opts.log {
        crate::logger::start("derlink", log_file)?;
    }

    let mut app = app::App::default();
    app.enable_start_stop_sync(!app.is_start_stop_sync_enabled());
    app.enable(!app.is_enabled());

    crate::app::run(terminal, &mut app, opts.fps)
}
