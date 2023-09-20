pub mod stream;

// #[cfg(feature = "ffi")]
pub mod ffi;

use crossbeam::channel::{Receiver, Sender};

pub type AudioBuffer = Vec<Vec<f32>>;

pub trait AudioReceiving: Sized {
    fn open_stream(&mut self) -> anyhow::Result<()>;
    fn list_devices(&self) -> anyhow::Result<Vec<String>>;
    fn select_device(&mut self, device_name: &str) -> anyhow::Result<()>;
    fn try_receive_audio(&mut self) -> anyhow::Result<AudioBuffer>;
}
