use super::*;
use cpal::{traits::*, FromSample, Sample, SizedSample};
use crossbeam::channel::{Receiver, Sender, TryRecvError};

pub struct HostedAudioOutput {
    host: cpal::Host,
    sender: Sender<AudioBuffer>,
    receiver: Receiver<AudioBuffer>,
    stream: AudioStream,
    devices: Vec<AudioDevice>,
}

impl Default for HostedAudioOutput {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(16);
        let host = cpal::default_host();

        Self {
            stream: AudioStream::default(),
            sender,
            receiver,
            devices: build_audio_device_list(&host),
            host,
        }
    }
}

impl AudioProviding for HostedAudioOutput {
    fn is_connected(&self) -> bool {
        self.stream.is_open()
    }

    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        if !channel_selection.is_valid_for_device(audio_device) {
            log::error!("Invalid selection : {channel_selection:?} for : {audio_device:#?}");
            return Ok(());
        }

        self.stream = self
            .host
            .input_devices()?
            .find(|device| device.name().ok().as_deref() == Some(&audio_device.name))
            .map(|device| AudioStream::open(self.sender.clone(), &device, channel_selection))
            .ok_or_else(|| anyhow::anyhow!("No audio device selected"))??;

        Ok(())
    }

    fn list_audio_devices(&self) -> &[AudioDevice] {
        self.devices.as_slice()
    }

    fn try_fetch_audio(&mut self) -> anyhow::Result<AudioBuffer> {
        match self.receiver.try_recv() {
            Ok(audio) => Ok(audio),
            Err(TryRecvError::Empty) => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }
}


