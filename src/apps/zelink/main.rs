mod state;
mod ui;

use crossterm::event::{self, Event, KeyCode};
use ratatui::prelude::*;
use std::time::{Duration, Instant};

pub fn run<B: Backend>(terminal: &mut Terminal<B>) -> anyhow::Result<()> {
    let mut state = state::AppState::with_on_state();

    let tick_rate = Duration::from_millis(33);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| ui::render(frame, &state))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                state.capture_session_state();
                match key.code {
                    KeyCode::Char('a') => state.enable(!state.is_enabled()),
                    KeyCode::Char('k') => {
                        state.set_session_tempo(state.tempo() + 1.0);
                        state.commit_session_state();
                    }
                    KeyCode::Char('j') => {
                        state.set_session_tempo(state.tempo() - 1.0);
                        state.commit_session_state();
                    }
                    KeyCode::Char('l') => state.set_quantum(state.quantum() + 1.),
                    KeyCode::Char('h') => state.set_quantum(state.quantum() - 1.),
                    KeyCode::Char('s') => {
                        state.enable_start_stop_sync(!state.is_start_stop_sync_enabled())
                    }
                    KeyCode::Char(' ') => {
                        state.toggle_session_is_playing();
                        state.commit_session_state();
                    }
                    KeyCode::Char('q') => {
                        state.stop();
                        return Ok(());
                    }
                    _ => (),
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            state.on_tick();
            last_tick = Instant::now();
        }
    }
}

termtools::main!(run);
