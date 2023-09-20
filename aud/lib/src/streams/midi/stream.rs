use super::*;
use crossbeam::channel::{Receiver, Sender};
use midir::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct HostedMidiReceiver {
    host: MidiInput,
    sender: Sender<MidiData>,
    receiver: Receiver<MidiData>,
    connection: Option<MidiInputConnection<Sender<MidiData>>>,
    is_running: Arc<AtomicBool>,
}

impl Default for HostedMidiReceiver {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(1_000);

        Self {
            host: MidiInput::new("aud-midi-in").unwrap(),
            connection: None,
            sender,
            receiver,
            is_running: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl MidiReceiving for HostedMidiReceiver {
    fn is_midi_stream_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    fn set_midi_stream_active(&mut self, should_be_active: bool) {
        self.is_running.store(should_be_active, Ordering::SeqCst)
    }

    fn connect_to_midi_device(&mut self, device_name: &str) -> anyhow::Result<()> {
        let ports = self.host.ports();
        let port = ports
            .iter()
            .find(|&port| self.host.port_name(port).as_deref() == Ok(device_name))
            .ok_or_else(|| anyhow::anyhow!("[ MIDI ] : Cannot find device {device_name}"))?;

        self.connection = Some(self.connect_to_input_device(port)?);
        log::trace!("[ MIDI ] : connected to {device_name}");
        Ok(())
    }

    fn list_midi_devices(&self) -> anyhow::Result<Vec<String>> {
        Ok(self
            .host
            .ports()
            .iter()
            .map(|port| self.host.port_name(port))
            .collect::<Result<Vec<_>, _>>()?)
    }

    fn try_receive_midi(&mut self) -> anyhow::Result<Vec<MidiData>> {
        Ok(self.receiver.try_iter().collect())
    }
}

impl HostedMidiReceiver {
    fn connect_to_input_device(
        &mut self,
        port: &MidiInputPort,
    ) -> anyhow::Result<MidiInputConnection<Sender<MidiData>>> {
        let callback = {
            let is_running = self.is_running.clone();

            move |timestamp: u64, bytes: &[u8], sender: &mut Sender<MidiData>| {
                if !is_running.load(Ordering::SeqCst) {
                    return;
                }

                let midi = MidiData {
                    timestamp,
                    bytes: bytes.into(),
                };

                if let Err(e) = sender.try_send(midi) {
                    log::error!("Failed to push midi message event to runtime : {e}");
                }
            }
        };

        MidiInput::new("aud-midi-in")?
            .connect(port, "aud-midi-in", callback, self.sender.clone())
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
}
