#[cfg(feature = "ffi")]
mod ffi;
mod input;
mod net;
mod stream;

pub use input::*;
pub use net::*;
pub use stream::*;

use serde::{Deserialize, Serialize};
use std::ops::Range;

pub trait AudioProviding {
    /// Determines if the audio source is currently accessible or connected.
    fn is_accessible(&self) -> bool;

    /// Lists available audio devices that this source can connect to.
    fn list_audio_devices(&self) -> &[AudioDevice];

    /// Attempts to establish a connection to a specified audio device for audio retrieval.
    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()>;

    /// Attempts to retrieve available audio data from this source.
    /// If the source is in a reachable state but
    /// there is no new audio data available,
    /// it should return an empty vector.
    /// This method should not block.
    fn retrieve_audio_buffer(&mut self) -> AudioBuffer;

    /// Processes any internal messages or events necessary to maintain the state
    /// of the audio source or to prepare for subsequent audio data retrieval.
    /// This method should be called periodically.
    fn process_audio_events(&mut self) -> anyhow::Result<()>;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    pub data: Vec<f32>,
    pub num_channels: u32,
}

impl AudioBuffer {
    pub fn new(num_samples: u32, num_channels: u32) -> Self {
        Self {
            data: vec![0.; num_samples as usize * num_channels as usize],
            num_channels,
        }
    }

    pub fn from_buffers(buffers: impl AsRef<[Self]>) -> Self {
        // Assume all buffers have the same number of channels
        let buffers = buffers.as_ref();
        let num_channels = buffers.first().map(|b| b.num_channels).unwrap_or(0);
        debug_assert!(buffers.iter().all(|buf| buf.num_channels == num_channels));
        let total_len: usize = buffers.iter().map(|buffer| buffer.data.len()).sum();
        let mut combined_data: Vec<_> = Vec::with_capacity(total_len);
        for buffer in buffers {
            combined_data.extend_from_slice(&buffer.data);
        }
        Self {
            data: combined_data,
            num_channels,
        }
    }

    pub fn from_deinterleaved(buffer: &[impl AsRef<[f32]>]) -> Self {
        Self {
            data: crate::dsp::interleave(buffer),
            num_channels: buffer.len() as u32,
        }
    }

    pub fn deinterleave(&self) -> Vec<Vec<f32>> {
        crate::dsp::deinterleave(&self.data, self.num_channels as usize)
    }

    pub fn num_frames(&self) -> usize {
        self.data.len() / self.num_channels as usize
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AudioDevice {
    pub name: String,
    pub num_channels: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AudioChannelSelection {
    Mono(usize),
    Stereo((usize, usize)),
    Range(Range<usize>),
    Multi(Vec<usize>),
}

impl AudioChannelSelection {
    pub fn to_vec(self) -> Vec<usize> {
        use AudioChannelSelection::*;

        match self {
            Mono(chan) => vec![chan],
            Stereo((a, b)) => vec![a, b],
            Range(r) => r.into_iter().collect(),
            Multi(list) => list,
        }
    }

    pub fn count(&self) -> usize {
        use AudioChannelSelection::*;

        match self {
            Mono(_) => 1,
            Stereo(_) => 2,
            Range(r) => r.end - r.start,
            Multi(list) => list.len(),
        }
    }

    pub fn is_valid_for_device(&self, device: &AudioDevice) -> bool {
        use AudioChannelSelection::*;

        let chans = 0..device.num_channels;
        match self {
            Mono(chan) => chans.contains(chan),
            Stereo((a, b)) => chans.contains(a) && chans.contains(b),
            Range(r) => r.contains(&chans.start) && !r.contains(&chans.end),
            Multi(list) => {
                list.iter().all(|chan| chans.contains(chan)) && list.len() < device.num_channels
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mono_channels_can_be_selected() {
        use AudioChannelSelection::*;

        let dev = AudioDevice {
            name: String::default(),
            num_channels: 1,
        };
        assert!(Mono(0).is_valid_for_device(&dev));

        for i in 1..100 {
            assert!(!Mono(i).is_valid_for_device(&dev));
        }
    }

    #[test]
    fn channel_selection_cannot_exceed_device_channel_count() {
        use AudioChannelSelection::*;

        const NUM_CHANNELS: usize = 8;
        let dev = AudioDevice {
            name: String::default(),
            num_channels: NUM_CHANNELS,
        };

        for i in 0..NUM_CHANNELS {
            assert!(Mono(i).is_valid_for_device(&dev));
            assert!(Stereo((i, i)).is_valid_for_device(&dev));
        }

        for i in NUM_CHANNELS..NUM_CHANNELS * 10 {
            assert!(!Mono(i).is_valid_for_device(&dev));
            assert!(!Stereo((i, i)).is_valid_for_device(&dev));
        }

        assert!(Range(0..NUM_CHANNELS).is_valid_for_device(&dev));
        assert!(!Range(NUM_CHANNELS..NUM_CHANNELS * 2).is_valid_for_device(&dev));
    }
}
