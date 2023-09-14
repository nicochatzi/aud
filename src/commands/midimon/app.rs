use crate::midi::*;
use crate::widgets::StatefulList;
use crossbeam::channel::{Receiver, Sender};
use crossterm::event::KeyCode;
use midir::MidiInputPorts;

const MAX_NUM_MESSAGES_ON_SCREEN: usize = 64;

pub struct App {
    pub selection: Option<String>,
    pub port_names: StatefulList<String>,
    pub messages: Vec<MidiMessageString>,
    pub is_running: bool,
    pub show_usage: bool,

    sender: Sender<MidiMessageString>,
    receiver: Receiver<MidiMessageString>,
    midi_in_connection: Option<midir::MidiInputConnection<Sender<MidiMessageString>>>,
    input_ports: MidiInputPorts,
}

impl Default for App {
    fn default() -> Self {
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
            show_usage: false,
        }
    }
}

impl App {
    pub fn update_ports(&mut self) -> anyhow::Result<()> {
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

    pub fn connect(&mut self) -> anyhow::Result<()> {
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
                                log::error!("failed to push midi message : {e}");
                            }
                        }
                    },
                    self.sender.clone(),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?,
        );

        Ok(())
    }

    pub fn collect(&mut self) {
        let mut new_messages: Vec<_> = self.receiver.try_iter().collect();
        if self.messages.len() > MAX_NUM_MESSAGES_ON_SCREEN {
            self.messages = self
                .messages
                .split_off(new_messages.len().min(self.messages.len() - 1));
        }
        self.messages.append(&mut new_messages);
    }
}

impl crate::app::Base for App {
    fn setup(&mut self) -> anyhow::Result<()> {
        self.update_ports()
    }

    fn update(&mut self) -> anyhow::Result<crate::app::Flow> {
        self.update_ports()?;

        if !self.is_running {
            return Ok(crate::app::Flow::Loop);
        }

        self.collect();

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
            KeyCode::Char('c') => self.messages.clear(),
            KeyCode::Char(' ') => self.is_running = !self.is_running,
            KeyCode::Down | KeyCode::Char('j') => self.port_names.next(),
            KeyCode::Up | KeyCode::Char('k') => self.port_names.previous(),
            KeyCode::Enter => {
                if self.port_names.selected().is_some() {
                    self.port_names.confirm_selection();
                    self.is_running = true;
                    self.messages.clear();
                    self.connect()?;
                }
            }
            _ => {}
        }

        Ok(crate::app::Flow::Continue)
    }
}
