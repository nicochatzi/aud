mod ui;

use aud::controllers::ableton_link::AbletonLink;
use crossterm::event::KeyCode;
use ratatui::prelude::*;

#[derive(Default)]
struct TerminalApp {
    ui: ui::Ui,
    app: AbletonLink,
}

impl crate::app::Base for TerminalApp {
    fn update(&mut self) -> anyhow::Result<crate::app::Flow> {
        self.app.capture_session_state();
        Ok(crate::app::Flow::Continue)
    }

    fn on_keypress(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<crate::app::Flow> {
        match key.code {
            KeyCode::Char('?') => self.ui.show_usage = !self.ui.show_usage,
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.ui.show_usage {
                    self.ui.show_usage = false
                } else {
                    self.app.stop();
                    return Ok(crate::app::Flow::Exit);
                }
            }
            KeyCode::Char('a') => self.app.enable(!self.app.is_enabled()),
            KeyCode::Char('k') => {
                self.app.set_session_tempo(self.app.tempo() + 1.0);
                self.app.commit_session_state();
            }
            KeyCode::Char('j') => {
                self.app.set_session_tempo(self.app.tempo() - 1.0);
                self.app.commit_session_state();
            }
            KeyCode::Char('l') => self.app.set_quantum(self.app.quantum() + 1.),
            KeyCode::Char('h') => self.app.set_quantum(self.app.quantum() - 1.),
            KeyCode::Char('s') => self
                .app
                .enable_start_stop_sync(!self.app.is_start_stop_sync_enabled()),
            KeyCode::Char(' ') => {
                self.app.toggle_session_is_playing();
                self.app.commit_session_state();
            }
            _ => (),
        }

        Ok(crate::app::Flow::Continue)
    }

    fn render(&mut self, f: &mut Frame) {
        self.ui.render(f, &mut self.app)
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
}

pub fn run(
    terminal: &mut Terminal<impl Backend>,
    opts: Options,
    common_opts: crate::CommonOptions,
) -> anyhow::Result<()> {
    if let Some(log_file) = opts.log.or_else(|| crate::locations::log_file("derlink")) {
        crate::logger::start("derlink", log_file, common_opts.verbose)?;
    }

    let mut app = TerminalApp::default();
    crate::app::run(terminal, &mut app, opts.fps.max(1.))
}
