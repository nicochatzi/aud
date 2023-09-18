mod app;
mod lua;
mod ui;

pub mod state;

use ratatui::prelude::*;

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

pub fn run(terminal: &mut Terminal<impl Backend>, opts: Options) -> anyhow::Result<()> {
    if let Some(log_file) = opts.log.or(crate::file::locations::log()) {
        crate::logger::start("midimon", log_file)?;
    }

    let mut app = app::App::new()?;

    if let Some(script) = opts.script {
        log::info!("{:#?}", script.canonicalize()?);
        app.set_scripts(script)?;
    }

    crate::app::run(terminal, &mut app, opts.fps)
}
