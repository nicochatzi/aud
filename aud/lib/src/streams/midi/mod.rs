mod stream;

pub use stream::*;

pub struct MidiData {
    pub timestamp: u64,
    pub bytes: Vec<u8>,
}
