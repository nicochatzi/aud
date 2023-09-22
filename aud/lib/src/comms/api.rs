//! We really want to avoid mixing serialized formats
//! so for the moment these are intentionally propagating
//! the bincode format requirement, instead of using serde::Seralize
//! traits in our generics.

use crate::audio::*;
use serde::{Deserialize, Serialize};

pub trait BincodeSerialize {
    fn serialize(self) -> Result<Vec<u8>, bincode::Error>;
}

pub trait BincodeDeserialize: Sized {
    fn deserialized(data: &[u8]) -> Result<Self, bincode::Error>;
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AudioResponse {
    Devices(Vec<AudioDevice>),
    Audio(AudioPacket),
}

impl BincodeSerialize for AudioResponse {
    fn serialize(self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(&self)
    }
}

impl BincodeDeserialize for AudioResponse {
    fn deserialized(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AudioRequest {
    GetDevices,
    Connect {
        device: AudioDevice,
        channels: AudioChannelSelection,
    },
}

impl BincodeSerialize for AudioRequest {
    fn serialize(self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(&self)
    }
}

impl BincodeDeserialize for AudioRequest {
    fn deserialized(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AudioPacket {
    pub metadata: AudioPacketMetadata,
    pub payload: AudioBuffer,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AudioPacketMetadata {
    pub index: u64,
    pub checksum: u32,
}

impl AudioPacket {
    pub fn new(index: u64, payload: AudioBuffer) -> Self {
        Self {
            metadata: AudioPacketMetadata {
                index,
                checksum: checksum(&payload),
            },
            payload,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.metadata.checksum == checksum(&self.payload)
    }
}

fn checksum(buffer: &AudioBuffer) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    for chan in buffer.iter() {
        hasher.update(unsafe {
            std::slice::from_raw_parts(
                chan.as_ptr() as *const u8,
                chan.len() * std::mem::size_of::<f32>(),
            )
        });
    }
    hasher.finalize()
}

#[derive(Default, Debug, Clone)]
pub struct AudioPacketSequence {
    packets: Vec<AudioPacket>,
}

impl AudioPacketSequence {
    pub fn push(&mut self, packet: AudioPacket) {
        if !packet.is_valid() {
            self.handle_missing_packet(packet);
            return;
        }

        self.insert_sorted(packet);
    }

    pub fn drain(&mut self) -> AudioBuffer {
        let mut audio = AudioBuffer::new();
        for packet in self.packets.drain(..) {
            audio.extend(packet.payload);
        }
        audio
    }

    fn handle_missing_packet(&mut self, packet: AudioPacket) {
        if self
            .packets
            .iter()
            .any(|p| p.metadata.index == packet.metadata.index)
        {
            return;
        }

        if let Some(last_good_packet) = self.packets.last() {
            self.insert_sorted(AudioPacket {
                metadata: packet.metadata,
                payload: last_good_packet.payload.clone(),
            });
        }
    }

    fn insert_sorted(&mut self, packet: AudioPacket) {
        match self
            .packets
            .binary_search_by_key(&packet.metadata.index, |p| p.metadata.index)
        {
            Ok(i) => {
                // replace the packet if it already exists
                self.packets[i] = packet;
            }
            Err(i) => {
                // insert the packet if it doesn't exist
                self.packets.insert(i, packet);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_packet_sequence_can_reoder_packets() {
        let mut seq = AudioPacketSequence::default();

        let packet1 = AudioPacket::new(1, vec![vec![0.0; 16]]);
        let packet3 = AudioPacket::new(3, vec![vec![1.0; 16]]);
        let packet2 = AudioPacket::new(2, vec![vec![2.0; 16]]);

        seq.push(packet1);
        seq.push(packet3);
        seq.push(packet2);

        let result = seq.drain();
        assert_eq!(result[0][0], 0.0);
        assert_eq!(result[1][0], 2.0);
        assert_eq!(result[2][0], 1.0);
    }

    #[test]
    fn audio_packet_sequence_can_fallback_to_last_valid_packet_if_checksum_fails() {
        let mut seq = AudioPacketSequence::default();

        let packet1 = AudioPacket::new(1, vec![vec![-1.0; 16]]);
        let mut packet2 = AudioPacket::new(2, vec![vec![1.0; 16]]);
        let packet3 = AudioPacket::new(3, vec![vec![2.0; 16]]);
        let packet4 = AudioPacket::new(4, vec![vec![3.0; 16]]);
        packet2.metadata.checksum += 1; // make it invalid

        seq.push(packet1);
        seq.push(packet2);
        seq.push(packet3);
        seq.push(packet4);

        let result = seq.drain();
        assert_eq!(result.len(), 4);
        assert_eq!(result[0][0], -1.0);
        assert_eq!(result[1][0], -1.0);
        assert_eq!(result[2][0], 2.0);
        assert_eq!(result[3][0], 3.0);
    }
}
