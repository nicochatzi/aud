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
