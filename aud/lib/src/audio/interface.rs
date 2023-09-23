use serde::{Deserialize, Serialize};
use std::{collections::HashSet, ops::Range};

pub trait AudioInterface {
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

    /// Process internal messages, this may include fetching
    /// or pushing audio to the underlying `AudioDevice`
    fn process_audio_events(&mut self) -> anyhow::Result<()>;
}

pub trait AudioProviding {
    /// Retrieve the latest available audio buffer.
    ///
    /// This method should not block.
    fn retrieve_audio_buffer(&mut self) -> AudioBuffer;
}

pub trait AudioConsuming {
    /// Consumes an `AudioBuffer` by pushing it into the currently connected `AudioDevice`.
    fn consume_audio_buffer(&mut self, buffer: AudioBuffer) -> anyhow::Result<()>;
}

/// An interleaved audio buffer.
///
/// Interleaved audio data means that the audio samples are
/// arranged in the buffer in a way that the samples for each
/// channel are alternated. For example, for stereo audio (2 channels),
/// the data would be arranged as [left0, right0, left1, right1, left2, right2, ...].
///
/// Note that the slice `[left0, right0]` is called a frame.
///
/// The API favors interleaved data since it is typically
/// what lower-level APIs use, and it is easier (and more compact)
/// for transferring or processing the audio data.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    pub data: Vec<f32>,
    pub num_channels: u32,
}

impl AudioBuffer {
    /// Creates a new `AudioBuffer` with a preallocated interleaved buffer.
    ///
    /// Each frame in the buffer contains one sample per channel. The total number of samples
    /// in the buffer is equal to `num_frames * num_channels`.
    ///
    /// # Parameters
    ///
    /// - `num_frames`: The number of frames in the buffer.
    /// - `num_channels`: The number of channels in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use aud_lib::audio::AudioBuffer;
    ///
    /// let buffer = AudioBuffer::with_frames(10, 2);
    /// assert_eq!(buffer.data.len(), 20);  // 10 frames * 2 channels
    /// assert_eq!(buffer.num_channels, 2);
    /// ```
    pub fn with_frames(num_frames: u32, num_channels: u32) -> Self {
        Self {
            data: vec![0.; num_frames as usize * num_channels as usize],
            num_channels,
        }
    }

    /// Creates a new `AudioBuffer` with a specified total length and number of channels.
    ///
    /// The length refers to the total number of samples in the buffer, which is equal to
    /// the product of the number of frames and the number of channels.
    ///
    /// # Parameters
    ///
    /// - `length`: The total number of samples in the buffer.
    /// - `num_channels`: The number of channels in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use aud_lib::audio::AudioBuffer;
    ///
    /// let buffer = AudioBuffer::with_len(20, 2);
    /// assert_eq!(buffer.data.len(), 20);  // Total number of samples
    /// assert_eq!(buffer.num_channels, 2);
    /// ```
    pub fn with_length(length: u32, num_channels: u32) -> Self {
        Self {
            data: vec![0.; length as usize],
            num_channels,
        }
    }

    /// Accumulate multiple audio buffers into a single larger buffer.
    ///
    /// This unsafely assumes all buffers have the same number of channels,
    /// it is up to the caller to guarantee this and pad is necessary.
    pub fn from_buffers(buffers: impl AsRef<[Self]>) -> Self {
        let buffers = buffers.as_ref();
        let num_channels = buffers.first().map(|b| b.num_channels).unwrap_or(0);
        debug_assert!(buffers.iter().all(|buf| buf.num_channels == num_channels));

        let total_len: usize = buffers.iter().map(|buffer| buffer.data.len()).sum();
        let mut buffer = Self::with_length(total_len as u32, num_channels); // Adjusted the length

        let mut start_idx = 0;
        buffers.iter().for_each(|buf| {
            let end_idx = start_idx + buf.data.len();
            buffer.data[start_idx..end_idx].copy_from_slice(&buf.data);
            start_idx = end_idx;
        });

        buffer
    }

    /// Create an interleaved `AudioBuffer` given a deinterleaved 2-D buffer.
    pub fn from_deinterleaved(buffer: &[impl AsRef<[f32]>]) -> Self {
        Self {
            data: crate::dsp::interleave(buffer),
            num_channels: buffer.len() as u32,
        }
    }

    /// Create an deinterleaved 2-D buffer from this interleaved `AudioBuffer`.
    pub fn deinterleave(&self) -> Vec<Vec<f32>> {
        crate::dsp::deinterleave(&self.data, self.num_channels as usize)
    }

    /// Number of "frames" in this interleaved buffer. This is effectively
    /// the same as "number of samples per channel" for this buffer.
    pub fn num_frames(&self) -> usize {
        self.data.len() / self.num_channels as usize
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AudioDevice {
    pub name: String,
    pub num_channels: usize,
}

impl AudioDevice {
    /// Check if the requested channel selection is viable for this device
    pub fn supports_channels(&self, selection: &AudioChannelSelection) -> bool {
        let chans = 0..self.num_channels;
        match selection {
            AudioChannelSelection::Mono(chan) => chans.contains(chan),
            AudioChannelSelection::Range(r) => r.contains(&chans.start) && !r.contains(&chans.end),
            AudioChannelSelection::Multi(list) => {
                list.iter().all(|chan| chans.contains(chan)) && list.len() < self.num_channels
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AudioChannelSelection {
    Mono(usize),
    Range(Range<usize>),
    Multi(Vec<usize>),
}

impl AudioChannelSelection {
    /// Build an array containing all the unique channel numbers
    pub fn to_vec(self) -> Vec<usize> {
        match self {
            AudioChannelSelection::Mono(chan) => vec![chan],
            AudioChannelSelection::Range(r) => r.into_iter().collect(),
            AudioChannelSelection::Multi(list) => list
                .into_iter()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect(),
        }
    }

    /// Count the number of unique channels selected
    pub fn count(&self) -> usize {
        match self {
            AudioChannelSelection::Mono(_) => 1,
            AudioChannelSelection::Range(r) => r.end - r.start,
            AudioChannelSelection::Multi(list) => {
                let unique_count: HashSet<_> = list.iter().cloned().collect();
                unique_count.len()
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
        assert!(dev.supports_channels(&Mono(0)));

        for i in 1..100 {
            assert!(!dev.supports_channels(&Mono(i)));
        }
    }

    #[test]
    fn range_of_channels_can_be_selected() {
        use AudioChannelSelection::*;

        const NUM_CHANNELS: usize = 8;
        let dev = AudioDevice {
            name: String::default(),
            num_channels: NUM_CHANNELS,
        };

        assert!(dev.supports_channels(&Range(0..NUM_CHANNELS)));
        assert!(!dev.supports_channels(&Range(NUM_CHANNELS..NUM_CHANNELS * 2)));
    }

    #[test]
    fn multiple_channels_can_be_selected() {
        use AudioChannelSelection::*;

        const NUM_CHANNELS: usize = 8;
        let dev = AudioDevice {
            name: String::default(),
            num_channels: NUM_CHANNELS,
        };

        assert!(dev.supports_channels(&Range(0..NUM_CHANNELS)));
        assert!(!dev.supports_channels(&Range(NUM_CHANNELS..NUM_CHANNELS * 2)));
    }
}
