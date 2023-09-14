pub mod audio;
pub mod commands;
pub mod midi;
pub mod widgets;

pub mod terminal {
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::prelude::*;

    pub fn acquire() -> anyhow::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
        let mut stdout = std::io::stdout();
        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        crossterm::terminal::enable_raw_mode()?;
        Ok(Terminal::new(backend::CrosstermBackend::new(stdout))?)
    }

    pub fn release() -> anyhow::Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    pub fn set_panic_hook() {
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            release().unwrap();
            original_hook(panic);
        }));
    }
}

/// Generate the main function given a terminal app runner:
///
/// ```
/// fn run<B: Backend>(_terminal: &mut Terminal<B>) -> anyhow::Result<()> {
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! main {
    ($app_entry: expr) => {
        pub fn main() -> anyhow::Result<()> {
            let mut terminal = $crate::terminal::acquire()?;
            $crate::terminal::set_panic_hook();

            let app_result = $app_entry(&mut terminal);

            $crate::terminal::release()?;

            if let Err(e) = app_result {
                log::error!("{e}");
            }

            Ok(())
        }
    };
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

    pub trait Base: Default {
        fn setup(&mut self) -> anyhow::Result<()> {
            Ok(())
        }

        fn update(&mut self) -> anyhow::Result<Flow> {
            Ok(Flow::Continue)
        }

        fn handle_key(&mut self, _key: KeyEvent) -> anyhow::Result<Flow> {
            Ok(Flow::Continue)
        }
    }

    pub fn run<A, B, F>(terminal: &mut Terminal<B>, render_ui: F) -> anyhow::Result<()>
    where
        A: Base,
        B: Backend,
        F: Fn(&mut Frame<B>, &mut A),
    {
        const TICK_RATE: Duration = Duration::from_millis(33);

        terminal.clear()?;

        let mut app = A::default();
        app.setup()?;

        let mut last_tick = Instant::now();

        loop {
            terminal.draw(|f| render_ui(f, &mut app))?;

            let timeout = TICK_RATE
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
                            _ => match app.handle_key(key)? {
                                Flow::Continue => (),
                                Flow::Loop => continue,
                                Flow::Exit => break,
                            },
                        }
                    }
                }
            }

            if last_tick.elapsed() >= TICK_RATE {
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
