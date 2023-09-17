use crossterm::event::KeyCode;
use ratatui::prelude::*;

const USAGE: &str = r#"
         ? : display help
  <ESC>, q : quit or hide help
     <C-c> : force quit
"#;

#[derive(Debug, clap::Parser)]
pub struct Options {
    /// Path to log file to write to
    #[arg(long)]
    log: Option<std::path::PathBuf>,

    /// Frames per second
    #[arg(long, default_value_t = 30.)]
    fps: f32,
}

struct App {
    is_running: bool,
    show_usage: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            is_running: true,
            show_usage: false,
        }
    }
}

impl crate::app::Base for App {
    fn update(&mut self) -> anyhow::Result<crate::app::Flow> {
        Ok(crate::app::Flow::Continue)
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<crate::app::Flow> {
        match key.code {
            KeyCode::Char('?') => self.show_usage = !self.show_usage,
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.show_usage {
                    self.show_usage = false
                } else {
                    return Ok(crate::app::Flow::Exit);
                }
            }
            _ => {}
        }

        Ok(crate::app::Flow::Continue)
    }

    fn render(&mut self, f: &mut Frame<impl Backend>) {
        if self.show_usage {
            crate::widgets::usage::render(f, USAGE);
        }
    }
}

pub fn run(terminal: &mut Terminal<impl Backend>, opts: Options) -> anyhow::Result<()> {
    if let Some(log_file) = opts.log {
        crate::logger::start("new_cmd", log_file)?;
    }

    let mut app = App::default();
    crate::app::run(terminal, &mut app, opts.fps)
}
