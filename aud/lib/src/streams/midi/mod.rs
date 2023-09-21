mod stream;

pub use stream::*;

pub struct MidiData {
    pub timestamp: u64,
    pub bytes: Vec<u8>,
}

pub trait MidiReceiving {
    fn is_midi_stream_active(&self) -> bool;
    fn set_midi_stream_active(&mut self, should_be_active: bool);

    fn list_midi_devices(&self) -> anyhow::Result<Vec<String>>;
    fn connect_to_midi_device(&mut self, device_name: &str) -> anyhow::Result<()>;
    fn produce_midi_messages(&mut self) -> Vec<MidiData>;
}
