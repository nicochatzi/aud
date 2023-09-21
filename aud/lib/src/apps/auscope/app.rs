use crate::audio::*;

pub struct App<AudioReceiver: AudioProviding> {
    audio_buffer: AudioBuffer,
    audio_receiver: AudioReceiver,
    audio_device: Option<AudioDevice>,
}

impl<AudioReceiver: AudioProviding> App<AudioReceiver> {
    pub fn with_audio_receiver(audio_receiver: AudioReceiver) -> Self {
        Self {
            audio_buffer: vec![],
            audio_receiver,
            audio_device: None,
        }
    }

    pub fn devices(&self) -> &[AudioDevice] {
        self.audio_receiver.list_audio_devices()
    }

    pub fn audio_mut(&mut self) -> &mut Vec<Vec<f32>> {
        &mut self.audio_buffer
    }

    pub fn update_channel_selection(
        &mut self,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        let Some(ref audio_device) = self.audio_device else {
            anyhow::bail!("No audio device selected");
        };

        self.audio_buffer.clear();
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
        self.update_channel_selection(channel_selection)
    }

    pub fn fetch_audio(&mut self) {
        while let Ok(mut channel_data) = self.audio_receiver.try_fetch_audio() {
            if channel_data.len() < self.audio_buffer.len() {
                self.audio_buffer
                    .resize(self.audio_buffer.len() - channel_data.len(), vec![]);
            }

            if channel_data.len() > self.audio_buffer.len() {
                let num_new_channels = channel_data.len() - self.audio_buffer.len();
                self.audio_buffer
                    .append(&mut vec![vec![]; num_new_channels]);
            }

            for (old_buf, new_buf) in self.audio_buffer.iter_mut().zip(channel_data.iter_mut()) {
                old_buf.append(new_buf);
            }
        }
    }
}
