pub mod commands;
pub mod ui;

pub mod terminal {
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::prelude::*;

    type CrossTerminal = Terminal<CrosstermBackend<std::io::Stdout>>;

    pub fn with_terminal<F>(f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut CrossTerminal) -> anyhow::Result<()>,
    {
        let mut terminal = acquire()?;
        set_panic_hook();
        f(&mut terminal)?;
        release()
    }

    fn acquire() -> anyhow::Result<CrossTerminal> {
        let mut stdout = std::io::stdout();
        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        crossterm::terminal::enable_raw_mode()?;

        let mut terminal = Terminal::new(backend::CrosstermBackend::new(stdout))?;
        terminal.hide_cursor()?;

        Ok(terminal)
    }

    fn release() -> anyhow::Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    fn set_panic_hook() {
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            // our Lua Runtime might panic, which is handled in "gracefully" in the app.
            // however, that panic will still trigger the global panic handler
            // so we need to specifically filter out panics originating from
            // the file that triggers the panic
            if let Some(location) = panic.location() {
                if location.file() == "src/lua/engine.rs" {
                    return;
                }
            }

            release().unwrap();
            original_hook(panic);
        }));
    }
}

pub mod app {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use ratatui::prelude::*;
    use std::time::{Duration, Instant};

    pub enum Flow {
        Continue,
        Loop,
        Exit,
    }

    pub trait Base {
        /// Called at terminal refresh rate
        fn update(&mut self) -> anyhow::Result<Flow> {
            Ok(Flow::Continue)
        }

        /// Called when a key press has been detected
        fn on_keypress(&mut self, _key: KeyEvent) -> anyhow::Result<Flow> {
            Ok(Flow::Continue)
        }

        /// Render the terminal UI frame
        fn render(&mut self, frame: &mut Frame<impl Backend>);
    }

    pub fn run(
        terminal: &mut Terminal<impl Backend>,
        app: &mut impl Base,
        fps: f32,
    ) -> anyhow::Result<()> {
        terminal.clear()?;

        let tick_rate = Duration::from_millis((1000. / fps) as u64);
        let mut last_tick = Instant::now();

        loop {
            terminal.draw(|f| app.render(f))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = crossterm::event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('c')
                                if matches!(key.modifiers, KeyModifiers::CONTROL) =>
                            {
                                return Ok(())
                            }
                            _ => match app.on_keypress(key)? {
                                Flow::Continue => (),
                                Flow::Loop => continue,
                                Flow::Exit => break,
                            },
                        }
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
                match app.update()? {
                    Flow::Continue => (),
                    Flow::Loop => continue,
                    Flow::Exit => break,
                }
            }
        }

        Ok(())
    }
}

pub mod logger {
    use std::{
        path::Path,
        sync::{
            atomic::{AtomicBool, Ordering},
            Once,
        },
    };

    static INIT: Once = Once::new();
    static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);

    pub fn is_active() -> bool {
        IS_INITIALIZED.load(Ordering::SeqCst)
    }

    pub fn start(id: &str, file: impl AsRef<Path>) -> anyhow::Result<()> {
        if is_active() {
            anyhow::bail!("attempted to setup logger more than once");
        }

        let id = format!("{}:{}", id.to_owned(), std::process::id());

        fern::Dispatch::new()
            .format(move |out, msg, record| {
                let time = humantime::format_rfc3339_seconds(std::time::SystemTime::now());

                if cfg!(debug_assertions) {
                    out.finish(format_args!(
                        "[ {id} ] : [ {time} ] : [ {} {} ] : {msg}",
                        record.target(),
                        record.level(),
                    ))
                } else {
                    out.finish(format_args!("[ {id} ] : [ {time} ] : {msg}"))
                }
            })
            .level(log::LevelFilter::Trace)
            .level_for("mio", log::LevelFilter::Off)
            .level_for("notify", log::LevelFilter::Off)
            .chain(fern::log_file(file.as_ref())?)
            .apply()?;

        log::trace!("started");

        INIT.call_once(|| IS_INITIALIZED.store(true, Ordering::SeqCst));
        Ok(())
    }
}
/// Default locations stored in `~/.aud`
///
/// .
/// ├── api
/// │  ├── aud/
/// │  ├── examples/
/// │  └── midimon/
/// ├── bin
/// │  └── aud
/// └── log
///    └── aud.log
///
pub mod locations {
    use std::path::PathBuf;

    pub fn aud() -> Option<PathBuf> {
        Some(dirs::home_dir()?.join(".aud"))
    }

    pub fn bin() -> Option<PathBuf> {
        Some(aud()?.join("bin"))
    }

    pub fn api() -> Option<PathBuf> {
        Some(aud()?.join("api"))
    }

    pub fn log() -> Option<PathBuf> {
        Some(aud()?.join("log"))
    }

    pub fn log_file() -> Option<std::path::PathBuf> {
        Some(log()?.join("aud.log"))
    }
}
