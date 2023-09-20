#[cfg(test)]
use mockall::{automock, mock, predicate::*};

pub mod stream;

// #[cfg(feature = "ffi")]
mod ffi;

use crossbeam::channel::{Receiver, Sender};
use std::ops::Range;

pub type AudioBuffer = Vec<Vec<f32>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDevice {
    pub name: String,
    pub channels: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioChannelSelection {
    Mono(usize),
    Stereo((usize, usize)),
    Range(Range<usize>),
    List(Vec<usize>),
}

impl AudioChannelSelection {
    pub fn to_vec(self) -> Vec<usize> {
        use AudioChannelSelection::*;

        match self {
            Mono(chan) => vec![chan],
            Stereo((a, b)) => vec![a, b],
            Range(r) => r.into_iter().collect(),
            List(list) => list,
        }
    }

    pub fn is_valid_for_device(&self, device: &AudioDevice) -> bool {
        use AudioChannelSelection::*;

        let chans = 0..device.channels;
        match self {
            Mono(chan) => chans.contains(chan),
            Stereo((a, b)) => chans.contains(a) && chans.contains(b),
            Range(r) => r.contains(&chans.start) && !r.contains(&chans.end),
            List(list) => {
                list.iter().all(|chan| chans.contains(chan)) && list.len() < device.channels
            }
        }
    }
}

#[cfg_attr(test, automock)]
pub trait AudioReceiving {
    fn list_audio_devices(&self) -> &[AudioDevice];

    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()>;

    fn try_receive_audio(&mut self) -> anyhow::Result<AudioBuffer>;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn channel_selection_cannot_exceed_device_channel_count() {
        use AudioChannelSelection::*;

        const NUM_CHANNELS: usize = 8;
        let dev = AudioDevice {
            name: String::default(),
            channels: NUM_CHANNELS,
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
