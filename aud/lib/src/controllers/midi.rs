use crate::{
    lua::{HostEvent, ScriptController},
    midi::{MidiData, MidiReceiving},
};
use std::{cell::RefCell, rc::Rc};

pub struct MidiReceiverController {
    receiver: Box<dyn MidiReceiving>,
    script: Rc<RefCell<ScriptController>>,
    port_names: Vec<String>,
    selected_port_name: Option<String>,
    messages: Vec<MidiData>,
}

impl MidiReceiverController {
    pub fn new(receiver: Box<dyn MidiReceiving>, script: Rc<RefCell<ScriptController>>) -> Self {
        Self {
            port_names: receiver.list_midi_devices().unwrap(),
            receiver,
            script,
            selected_port_name: None,
            messages: vec![],
        }
    }

    pub fn is_running(&self) -> bool {
        self.receiver.is_midi_stream_active()
    }

    pub fn set_running(&mut self, should_run: bool) {
        self.receiver.set_midi_stream_active(should_run)
    }

    pub fn port_names(&self) -> &[String] {
        self.port_names.as_slice()
    }

    pub fn selected_port_name(&self) -> Option<&str> {
        self.selected_port_name.as_deref()
    }

    pub fn push_message(&mut self, message: MidiData) {
        self.messages.push(message)
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    pub fn take_messages(&mut self) -> Vec<MidiData> {
        std::mem::take(&mut self.messages)
    }

    /// Transfer all received MIDI messages to the engine.
    pub fn update(&mut self) {
        for msg in self.receiver.produce_midi_messages() {
            if let Err(e) = self.script.borrow().try_send(HostEvent::Midi(msg)) {
                log::error!("Failed to send midi to Lua Runtime : {e}");
            }
        }
    }

    pub fn reconnect(&mut self) -> anyhow::Result<()> {
        if self.selected_port_name.is_some() {
            let port = self.selected_port_name.as_ref().unwrap().clone();
            self.connect_to_input(&port)?;
        }

        Ok(())
    }

    pub fn connect_to_input(&mut self, port_name: &str) -> anyhow::Result<()> {
        let Some(index) = self.port_names.iter().position(|name| name == port_name) else {
            anyhow::bail!("port not found : {port_name}")
        };

        self.connect_to_input_by_index(index)
    }

    pub fn connect_to_input_by_index(&mut self, index: usize) -> anyhow::Result<()> {
        let Some(port_name) = self.port_names.get(index) else {
            anyhow::bail!("invalid port selection : {index}");
        };

        let port_name = port_name.to_owned();
        self.connect_to_input_unchecked(port_name)?;
        self.clear_messages();
        Ok(())
    }

    fn connect_to_input_unchecked(&mut self, port_name: String) -> anyhow::Result<()> {
        self.receiver.connect_to_midi_device(&port_name)?;
        self.selected_port_name = Some(port_name.clone());

        if let Err(e) = self.script.borrow().try_send(HostEvent::Connect(port_name)) {
            log::error!("Failed to send device connected event to runtime : {e}");
        }

        Ok(())
    }
}
