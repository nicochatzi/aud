pub struct MidiReceiverController {
    receiver: Box<dyn MidiReceiving>,
    script: Rc<ScriptController>,
    port_names: Vec<String>,
    selected_port_name: Option<String>,
    messages: Vec<MidiData>,
}

impl MidiReceiverController {
    pub fn new(receiver: Box<dyn MidiReceiving>, script: Rc<ScriptController>) -> Self {
        Self {
            port_names: receiver.list_midi_devices().unwrap(),
            receiver,
            script,
            selected_port_name: None,
            messages: vec![],
        }
    }

    pub fn is_runing(&self) -> bool {
        self.receiver.is_midi_stream_active()
    }

    pub fn set_running(&mut self, should_run: bool) {
        self.receiver.set_midi_stream_active(should_run)
    }

    pub fn midi_ports(&self) -> &[String] {
        self.port_names.as_slice()
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    pub fn selected_port(&self) -> Option<&str> {
        self.selected_port_name.as_deref()
    }

    pub fn take_messages(&mut self) -> Vec<MidiData> {
        std::mem::take(&mut self.messages)
    }

    pub fn connect_to_input_by_index(&mut self, port_index: usize) -> anyhow::Result<()> {
        if self.port_names.get(port_index).is_none() {
            return Ok(());
        }
        self.connect_to_midi_input_by_index_unchecked(port_index)?;
        self.clear_messages();
        Ok(())
    }

    pub fn connect_to_input(&mut self, port_name: &str) -> anyhow::Result<()> {
        match self.port_names.iter().position(|name| name == port_name) {
            Some(index) => self.connect_to_midi_input_by_index(index),
            None => anyhow::bail!("port not found : {port_name}"),
        }
    }

    fn connect_to_input_by_index_unchecked(&mut self, index: usize) -> anyhow::Result<()> {
        let port_name = &self.midi_port_names[index];
        self.receiver.connect_to_midi_device(port_name)?;
        self.selected_port_name = Some(port_name.into());
        let port_name = port_name.to_owned();

        if let Err(e) = self.script.try_send(HostEvent::Connect(port_name)) {
            log::error!("Failed to send device connected event to runtime : {e}");
        }

        Ok(())
    }
}
