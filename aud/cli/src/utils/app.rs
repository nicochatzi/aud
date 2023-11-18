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
    fn render(&mut self, frame: &mut Frame);
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
                        KeyCode::Char('c') if matches!(key.modifiers, KeyModifiers::CONTROL) => {
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
