pub mod apps;
pub mod audio;
pub mod comms;
pub mod files;
pub mod lua;
pub mod midi;

pub mod dsp {
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

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_deinterleave() {
            let input = &[0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
            let expected_output = &[&[0.1, 0.3, 0.5], &[0.2, 0.4, 0.6]];
            let output = deinterleave(input, 2);
            assert_eq!(output, expected_output);
        }

        #[test]
        fn test_interleave() {
            let input = &[&[0.1, 0.3, 0.5], &[0.2, 0.4, 0.6]];
            let expected_output = &[0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
            let output = interleave(input);
            assert_eq!(output, expected_output);
        }
    }
}
