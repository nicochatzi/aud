//! We really want to avoid mixing serialized formats
//! so for the moment these are intentionally propagating
//! the bincode format requirement, instead of using serde::Seralize
//! traits in our generics.

use crate::streams::audio::*;
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
    Audio(AudioBuffer),
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
