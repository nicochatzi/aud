use crate::audio::*;

pub struct App<AudioReceiver> {
    audio_buffer: AudioBuffer,
    audio_receiver: AudioReceiver,
    audio_device: Option<AudioDevice>,
}

impl<AudioReceiver> App<AudioReceiver>
where
    AudioReceiver: AudioProviding + AudioInterface,
{
    pub fn with_audio_receiver(audio_receiver: AudioReceiver) -> Self {
        Self {
            audio_buffer: AudioBuffer::default(),
            audio_receiver,
            audio_device: None,
        }
    }

    pub fn devices(&self) -> &[AudioDevice] {
        self.audio_receiver.list_audio_devices()
    }

    pub fn audio_mut(&mut self) -> &mut AudioBuffer {
        &mut self.audio_buffer
    }

    pub fn update_channel_selection(
        &mut self,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        let Some(ref audio_device) = self.audio_device else {
            anyhow::bail!("No audio device selected");
        };

        self.audio_buffer.data.clear();
        self.audio_receiver
            .connect_to_audio_device(audio_device, channel_selection)?;
        self.audio_device = Some(audio_device.clone());
        Ok(())
    }

    pub fn connect_to_audio_input(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        self.audio_device = Some(audio_device.clone());
        self.audio_buffer.num_channels = channel_selection.count() as u32;
        self.update_channel_selection(channel_selection)
    }

    pub fn fetch_audio(&mut self) -> anyhow::Result<()> {
        self.audio_receiver.process_audio_events()?;

        let mut audio = self.audio_receiver.retrieve_audio_buffer();
        debug_assert_eq!(self.audio_buffer.num_channels, audio.num_channels);
        self.audio_buffer.data.append(&mut audio.data);

        Ok(())
    }
}
