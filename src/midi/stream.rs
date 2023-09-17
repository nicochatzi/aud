use midir::*;

pub struct Input<T: 'static> {
    midi: MidiInput,
    selected_port: Option<String>,
    connection: Option<MidiInputConnection<T>>,
}

impl<T> Default for Input<T> {
    fn default() -> Self {
        Self {
            midi: MidiInput::new("aud-midi-in").unwrap(),
            selected_port: None,
            connection: None,
        }
    }
}

impl<T: 'static + Send> Input<T> {
    pub fn ports(&self) -> MidiInputPorts {
        self.midi.ports()
    }

    pub fn select(&mut self, port_name: &str) -> anyhow::Result<()> {
        self.selected_port = Some(port_name.to_string());
        Ok(())
    }

    pub fn selection(&self) -> Option<String> {
        self.selected_port.clone()
    }

    pub fn names(&mut self) -> anyhow::Result<Vec<String>> {
        let ports = self.ports();
        let mut names = Vec::with_capacity(ports.len());
        for port in &ports {
            names.push(self.midi.port_name(port)?);
        }
        Ok(names)
    }

    pub fn connect<F>(&mut self, callback: F, data: T) -> anyhow::Result<()>
    where
        F: FnMut(u64, &[u8], &mut T) + Send + 'static,
    {
        let Some(ref port_name) = self.selected_port else {
            return Ok(());
        };

        let ports = self.ports();
        let Some(port) = ports.iter().find(|port| {
            if let Ok(name) = self.midi.port_name(port) {
                &name == port_name
            } else {
                false
            }
        }) else {
            anyhow::bail!("Invalid port selection : {port_name}");
        };

        self.connection = Some(
            MidiInput::new("aud-midi-in")?
                .connect(port, "aud-read-input", callback, data)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?,
        );

        log::trace!("MIDI In connected : {port_name}");

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }
}

pub struct Output {
    midi: MidiOutput,
    selected_port: Option<String>,
    connection: Option<MidiOutputConnection>,
}

impl Default for Output {
    fn default() -> Self {
        Self {
            midi: MidiOutput::new("aud-midi-out").unwrap(),
            selected_port: None,
            connection: None,
        }
    }
}

impl Output {
    pub fn ports(&self) -> MidiOutputPorts {
        self.midi.ports()
    }

    pub fn select(&mut self, port_name: &str) -> anyhow::Result<()> {
        self.selected_port = Some(port_name.to_string());
        Ok(())
    }

    pub fn selection(&self) -> Option<String> {
        self.selected_port.clone()
    }

    pub fn names(&mut self) -> anyhow::Result<Vec<String>> {
        let ports = self.ports();
        let mut names = Vec::with_capacity(ports.len());
        for port in &ports {
            names.push(self.midi.port_name(port)?);
        }
        Ok(names)
    }

    pub fn connect(&mut self) -> anyhow::Result<()> {
        let Some(ref port_name) = self.selected_port else {
            return Ok(());
        };

        let ports = self.ports();
        let Some(port) = ports.iter().find(|port| {
            if let Ok(name) = self.midi.port_name(port) {
                &name == port_name
            } else {
                false
            }
        }) else {
            anyhow::bail!("Invalid port selection : {port_name}");
        };

        let Ok(connection) = MidiOutput::new("aud-midi-out")?.connect(port, "aud-midi-out") else {
            anyhow::bail!("Failed to connect to midi output");
        };

        log::trace!("MIDI Out connected : {port_name}");

        self.connection = Some(connection);

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    pub fn send(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        if let Some(ref mut connection) = &mut self.connection {
            connection.send(bytes)?;
        }

        Ok(())
    }
}
