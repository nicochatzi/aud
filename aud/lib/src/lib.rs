#![warn(unused_extern_crates)]
#![warn(rust_2018_idioms)]
#![warn(rust_2021_incompatible_or_patterns)]
#![warn(rust_2021_incompatible_closure_captures)]

pub mod apps;
pub mod audio;
pub mod comms;
pub mod files;
pub mod lua;
pub mod midi;

pub mod dsp {
    /// Deinterleaves a single buffer into multiple channel buffers.
    ///
    /// The input buffer is expected to have interleaved audio samples, where channels' samples are
    /// alternated. This function reorganizes those samples into separate buffers for each channel.
    ///
    /// # Parameters
    /// - `buffer`: The input buffer containing the interleaved audio data.
    /// - `num_channels`: The number of channels in the interleaved audio data.
    ///
    /// # Returns
    /// A `Vec` containing separate `Vec<f32>` buffers for each channel.
    ///
    /// # Examples
    /// ```rust
    /// use aud_lib::dsp::deinterleave;
    ///
    /// let interleaved = vec![1.0, 2.0, 3.0, 4.0];  // Assuming 2 channels
    /// let deinterleaved = deinterleave(&interleaved, 2);
    /// assert_eq!(deinterleaved, vec![vec![1.0, 3.0], vec![2.0, 4.0]]);
    /// ```
    #[inline]
    pub fn deinterleave(buffer: &[f32], num_channels: usize) -> Vec<Vec<f32>> {
        let num_samples = buffer.len() / num_channels;
        let mut out = vec![vec![0.; num_samples]; num_channels];

        for channel in 0..num_channels {
            for sample in 0..num_samples {
                out[channel][sample] = buffer[sample * num_channels + channel];
            }
        }

        out
    }

    /// Interleaves multiple channel buffers into a single buffer.
    ///
    /// The input is a slice of buffers, each containing the audio samples for a single channel.
    /// This function reorganizes those samples into an interleaved buffer where channels' samples are alternated.
    ///
    /// # Parameters
    /// - `buffer`: The input slice containing references to the channel buffers.
    ///
    /// # Returns
    /// A `Vec<f32>` containing the interleaved audio data.
    ///
    /// # Examples
    /// ```rust
    /// use aud_lib::dsp::interleave;
    ///
    /// let channels = vec![vec![1.0, 3.0], vec![2.0, 4.0]];
    /// let interleaved: Vec<_> = channels.iter().map(AsRef::as_ref).collect();
    /// let interleaved_buffer = interleave(&interleaved);
    /// assert_eq!(interleaved_buffer, vec![1.0, 2.0, 3.0, 4.0]);
    /// ```
    #[inline]
    pub fn interleave(buffer: &[impl AsRef<[f32]>]) -> Vec<f32> {
        let num_channels = buffer.len();

        if num_channels == 0 {
            return vec![];
        }

        let num_samples = buffer[0].as_ref().len();
        let mut out = vec![0.; num_channels * num_samples];

        for channel in 0..num_channels {
            for sample in 0..num_samples {
                out[sample * num_channels + channel] = buffer[channel].as_ref()[sample];
            }
        }

        out
    }
}
