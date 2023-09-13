mod midi;
mod ui;

pub use midi::*;

use crossbeam::channel::{Receiver, Sender};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use midir::MidiInputPorts;
use ratatui::prelude::*;
use std::time::{Duration, Instant};
use termtools::StatefulList;

const TICK_RATE: Duration = Duration::from_millis(33);
const MAX_NUM_MESSAGES_ON_SCREEN: usize = 64;

pub struct State {
    pub selection: Option<String>,
    pub port_names: StatefulList<String>,
    pub messages: Vec<MidiMessageString>,
    sender: Sender<MidiMessageString>,
    receiver: Receiver<MidiMessageString>,
    midi_in_connection: Option<midir::MidiInputConnection<Sender<MidiMessageString>>>,
    input_ports: MidiInputPorts,
    is_running: bool,
}

impl State {
    fn new() -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(1_000);

        Self {
            selection: None,
            sender,
            receiver,
            midi_in_connection: None,
            port_names: StatefulList::default(),
            input_ports: vec![],
            messages: vec![],
            is_running: true,
        }
    }

    fn update_ports(&mut self) -> anyhow::Result<()> {
        let midi_in = midir::MidiInput::new("midir reading input")?;
        self.input_ports = midi_in.ports();
        let mut input_port_names = Vec::with_capacity(self.input_ports.len());
        for port in &self.input_ports {
            input_port_names.push(midi_in.port_name(port)?);
        }

        if input_port_names != self.port_names.items {
            self.port_names = StatefulList::with_items(input_port_names);
        }

        Ok(())
    }

    fn connect(&mut self) -> anyhow::Result<()> {
        let Some(index) = self.port_names.selected() else {
            return Ok(());
        };

        let Some(input_port) = self.input_ports.get(index) else {
            anyhow::bail!("Invalid port selection");
        };

        let midi_in = midir::MidiInput::new("midir reading input")?;
        self.selection = Some(midi_in.port_name(input_port)?);
        self.midi_in_connection = Some(
            midi_in
                .connect(
                    input_port,
                    "midir-read-input",
                    move |timestamp, bytes, sender| {
                        if let Some(msg) = MidiMessageString::new(timestamp, bytes) {
                            if let Err(e) = sender.try_send(msg) {
                                println!("failed to push midi message : {e}");
                            }
                        }
                    },
                    self.sender.clone(),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?,
        );

        Ok(())
    }

    fn collect(&mut self) {
        let mut new_messages: Vec<_> = self.receiver.try_iter().collect();
        if self.messages.len() > MAX_NUM_MESSAGES_ON_SCREEN {
            self.messages = self
                .messages
                .split_off(new_messages.len().min(self.messages.len() - 1));
        }
        self.messages.append(&mut new_messages);
    }
}

fn run<B: Backend>(terminal: &mut Terminal<B>) -> anyhow::Result<()> {
    terminal.clear()?;

    let mut state = State::new();
    state.update_ports()?;

    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, &mut state))?;

        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('c') if matches!(key.modifiers, KeyModifiers::CONTROL) => {
                            break
                        }
                        KeyCode::Char('c') => state.messages.clear(),
                        KeyCode::Char(' ') => state.is_running = !state.is_running,
                        KeyCode::Down | KeyCode::Char('j') => state.port_names.next(),
                        KeyCode::Up | KeyCode::Char('k') => state.port_names.previous(),
                        KeyCode::Enter => {
                            if state.port_names.selected().is_some() {
                                state.is_running = true;
                                state.messages.clear();
                                state.connect()?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            last_tick = Instant::now();
            state.update_ports()?;

            if !state.is_running {
                continue;
            }

            state.collect();
        }
    }

    Ok(())
}

termtools::main!(run);
