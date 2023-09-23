use crate::audio::*;
use serde::{Deserialize, Serialize};

impl AudioBuffer {
    fn checksum(&self) -> u32 {
        let chans_crc = crc32fast::hash(&self.num_channels.to_le_bytes());
        let data_crc = crc32fast::hash(unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr() as *const u8,
                self.data.len() * std::mem::size_of::<f32>(),
            )
        });
        chans_crc.wrapping_add(data_crc)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AudioPacket {
    pub header: AudioPacketHeader,
    pub buffer: AudioBuffer,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct AudioPacketHeader {
    pub index: u64,
    pub checksum: u32,
}

impl AudioPacket {
    pub fn new(index: u64, buffer: &impl AsRef<[f32]>, num_channels: u32) -> Self {
        let buffer = AudioBuffer {
            data: buffer.as_ref().to_owned(),
            num_channels,
        };

        Self {
            header: AudioPacketHeader {
                index,
                checksum: buffer.checksum(),
            },
            buffer,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.header.checksum == self.buffer.checksum()
    }
}

/// A sequence of audio packets that can reorder
/// packets and has a simple invalid packet
/// replacement mechanism.
///
#[derive(Default, Debug, Clone)]
pub struct AudioPacketSequence {
    packets: Vec<AudioPacket>,
}

impl AudioPacketSequence {
    pub const NUM_BUFFER_PACKETS: usize = 4;
    const NUM_SAMPLES_PER_PACKET: usize = 256;

    /// Create a sequence given a multi-channel audio buffer
    /// This can then be used to stream by extracting the packets,
    /// which is why this struct itself is not serialisable
    /// while individual packets are.
    pub fn from_buffer(start_index: u64, buffer: &AudioBuffer) -> Self {
        Self {
            packets: buffer
                .data
                .chunks(Self::NUM_SAMPLES_PER_PACKET)
                .enumerate()
                .map(|(i, chunk)| {
                    AudioPacket::new(start_index + i as u64, &chunk, buffer.num_channels)
                })
                .collect(),
        }
    }

    /// Batch sequence construction by filtered invalid
    /// packets and sorting the sequence once. If the
    /// caller implements packet buffering this
    /// can be used to efficiently create the sequence.
    pub fn with_packets(mut packets: Vec<AudioPacket>) -> Self {
        if packets.is_empty() {
            return Self::default();
        }

        let num_channels = packets[0].buffer.num_channels;
        packets.retain(|p| p.is_valid() && p.buffer.num_channels == num_channels);

        let mut seq = Self { packets };
        seq.sort();
        seq
    }

    /// Consume the sequence returning the raw array of packets
    pub fn into_packets(self) -> Vec<AudioPacket> {
        self.packets
    }

    /// Push a single packet and update the internal sequence.
    /// When possible, prefer batch operations for performance.
    pub fn push(&mut self, packet: AudioPacket) {
        if !packet.is_valid() {
            log::warn!("invalid packet checksum");
            return;
        }

        let receiving_different_channel_count = self
            .packets
            .last()
            .is_some_and(|p| p.buffer.num_channels != packet.buffer.num_channels);

        if receiving_different_channel_count {
            self.packets.clear();
            return;
        }

        self.insert_sorted(packet);
    }

    /// Total number of packets currently held in the sequence.
    pub fn num_packets(&self) -> usize {
        self.packets.len()
    }

    /// Get the number of channels in each buffer
    /// has in this sequence.
    pub fn num_channels(&self) -> u32 {
        self.packets
            .first()
            .map_or(0, |packet| packet.buffer.num_channels)
    }

    /// Fast way to query the total number of
    /// frames that can be extracted using
    /// `extract()`.
    ///
    /// Note this is the total number of samples
    /// _per channel per buffer_ for interleaved
    /// audio
    pub fn num_available_frames(&self) -> usize {
        self.packets
            .iter()
            .take(self.packets.len().saturating_sub(Self::NUM_BUFFER_PACKETS))
            .map(|p| p.buffer.num_frames())
            .sum()
    }

    /// Consumes the entire sequence, returning each
    /// packet as an individual `AudioBuffer`.
    ///
    /// This is used to immediately extract all buffers
    /// immediately, i.e. without buffering/latency.
    pub fn consume(&mut self) -> Vec<AudioBuffer> {
        self.drain(self.packets.len())
    }

    /// Safe extraction which retains enough buffers internally
    /// to help maintain packet ordering.
    ///
    /// However it will add latency of at most:
    ///     NUM_BUFFER_PACKETS * SAMPLES_PER_PACKET
    pub fn extract(&mut self) -> Vec<AudioBuffer> {
        self.drain(Self::NUM_BUFFER_PACKETS.min(self.packets.len()))
    }

    fn drain(&mut self, num_packets: usize) -> Vec<AudioBuffer> {
        self.packets
            .drain(..num_packets.min(self.packets.len()))
            .map(|p| p.buffer)
            .collect()
    }

    fn sort(&mut self) {
        self.packets
            .sort_by(|a, b| a.header.index.cmp(&b.header.index));
    }

    fn insert_sorted(&mut self, packet: AudioPacket) {
        match self
            .packets
            .binary_search_by(|p| p.header.index.cmp(&packet.header.index))
        {
            Ok(i) => self.packets.insert(i, packet), // replace the packet at index if it already exists
            Err(i) => self.packets.insert(i, packet), // insert the packet at the correct position to maintain order
        }
    }
}

#[derive(Default)]
pub struct AudioPacketSequenceBuilder {
    packet_count: u64,
}

impl AudioPacketSequenceBuilder {
    pub fn from_buffer(&mut self, buffer: &AudioBuffer) -> AudioPacketSequence {
        let sequence = AudioPacketSequence::from_buffer(self.packet_count, buffer);
        self.packet_count += sequence.num_packets() as u64;
        sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_packet_sequence_will_split_a_buffer_into_correct_number_of_packets() {
        const NUM_EXPECTED_PACKETS: u32 = 16;
        const NUM_SAMPLES: u32 =
            AudioPacketSequence::NUM_SAMPLES_PER_PACKET as u32 * NUM_EXPECTED_PACKETS;
        let packets =
            AudioPacketSequence::from_buffer(0, &AudioBuffer::with_length(NUM_SAMPLES, 1))
                .into_packets();

        assert_eq!(packets.len(), NUM_EXPECTED_PACKETS as usize);
    }

    #[test]
    fn audio_packet_sequence_will_return_a_buffer_per_packet() {
        let num_channels = 1;
        let num_fragments_per_buffer = 8;
        let num_samples = AudioPacketSequence::NUM_SAMPLES_PER_PACKET * num_fragments_per_buffer;

        let expected_buffer = AudioBuffer {
            data: (0..num_samples * num_channels)
                .into_iter()
                .map(|x| x as f32)
                .collect(),
            num_channels: num_channels as u32,
        };

        let packets = AudioPacketSequence::from_buffer(0, &expected_buffer).into_packets();
        let buffers = AudioPacketSequence::with_packets(packets).consume();
        assert_eq!(buffers.len(), num_fragments_per_buffer);

        let combined_buffer = AudioBuffer::from_buffers(buffers);
        assert_eq!(combined_buffer.data.len(), expected_buffer.data.len());

        for (returned, expected) in combined_buffer.data.iter().zip(expected_buffer.data.iter()) {
            assert_eq!(returned, expected);
        }
    }

    #[test]
    fn audio_packet_sequence_can_reorder_packets() {
        const NUM_CHANNELS: usize = 16;

        let packet1 = AudioPacket::new(1, &vec![0.0; NUM_CHANNELS], 1);
        let packet2 = AudioPacket::new(2, &vec![1.0; NUM_CHANNELS], 1);
        let packet3 = AudioPacket::new(3, &vec![2.0; NUM_CHANNELS], 1);

        let buffer = AudioPacketSequence::with_packets(vec![packet1, packet3, packet2]).consume();

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer[0].data[0], 0.0);
        assert_eq!(buffer[1].data[0], 1.0);
        assert_eq!(buffer[2].data[0], 2.0);
    }

    #[test]
    fn audio_packet_sequence_handles_invalid_packets() {
        let packet1 = AudioPacket::new(1, &vec![-1.0; 4], 1);
        let packet2 = AudioPacket::new(1, &vec![-1.0; 4], 1);
        let mut packet3 = AudioPacket::new(2, &vec![1.0; 4], 1);
        packet3.header.checksum = packet3.header.checksum.wrapping_add(1);

        let buffers = AudioPacketSequence::with_packets(vec![packet1, packet2, packet3]).consume();

        assert_eq!(buffers.len(), 2);
        assert!(buffers.iter().all(|b| b.data.iter().all(|x| *x == -1.)));
    }
}
