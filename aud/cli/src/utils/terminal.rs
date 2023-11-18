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
