use crate::{
    audio::{AudioBuffer, AudioChannelSelection, AudioDevice, AudioInterface, AudioProviding},
    lua::{HostEvent, ScriptController},
};
use std::{cell::RefCell, rc::Rc};

pub trait AudioProvider: AudioProviding + AudioInterface {}

impl<T> AudioProvider for T where T: AudioProviding + AudioInterface {}

pub struct AudioProviderController {
    receiver: Box<dyn AudioProvider>,
    script: Rc<RefCell<ScriptController>>,
    buffer: AudioBuffer,
    selected_device: Option<AudioDevice>,
    selected_channels: Option<AudioChannelSelection>,
}

impl AudioProviderController {
    pub fn new(receiver: Box<dyn AudioProvider>, script: Rc<RefCell<ScriptController>>) -> Self {
        Self {
            buffer: AudioBuffer::default(),
            receiver,
            script,
            selected_device: None,
            selected_channels: None,
        }
    }

    pub fn devices(&self) -> &[AudioDevice] {
        self.receiver.list_audio_devices()
    }

    pub fn buffer(&self) -> &AudioBuffer {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut AudioBuffer {
        &mut self.buffer
    }

    pub fn selected_device(&self) -> Option<&AudioDevice> {
        self.selected_device.as_ref()
    }

    pub fn selected_channels(&self) -> Option<&AudioChannelSelection> {
        self.selected_channels.as_ref()
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        self.receiver.process_audio_events()?;
        let mut audio = self.receiver.retrieve_audio_buffer();
        if self.buffer.num_channels != audio.num_channels {
            self.buffer = audio;
        } else {
            self.buffer.data.append(&mut audio.data);
        }
        Ok(())
    }

    pub fn reconnect(&mut self) -> anyhow::Result<()> {
        if self.selected_device().is_some() && self.selected_channels().is_some() {
            self.connect_to_input(
                &self.selected_device.as_ref().unwrap().clone(),
                self.selected_channels.as_ref().unwrap().clone(),
            )?;
        }

        Ok(())
    }

    pub fn update_channel_selection(
        &mut self,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        let Some(ref audio_device) = self.selected_device else {
            anyhow::bail!("No audio device selected");
        };

        self.buffer.data.clear();
        self.receiver
            .connect_to_audio_device(audio_device, channel_selection.clone())?;
        self.selected_channels = Some(channel_selection);

        if let Err(e) = self
            .script
            .borrow()
            .try_send(HostEvent::Connect(audio_device.name.clone()))
        {
            log::error!("Failed to send device connected event to runtime : {e}");
        }

        Ok(())
    }

    pub fn connect_to_input(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        self.selected_device = Some(audio_device.clone());
        self.buffer.num_channels = channel_selection.count() as u32;
        self.update_channel_selection(channel_selection)
    }
}
