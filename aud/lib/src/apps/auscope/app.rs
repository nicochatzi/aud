use crate::streams::audio::AudioReceiving;

pub struct App<AudioReceiver: AudioReceiving> {
    device_names: Vec<String>,
    audio_buffer: Vec<Vec<f32>>,
    audio_receiver: AudioReceiver,
}

impl<AudioReceiver: AudioReceiving> App<AudioReceiver> {
    pub fn with_audio_receiver(audio_receiver: AudioReceiver) -> Self {
        Self {
            device_names: vec![],
            audio_buffer: vec![],
            audio_receiver,
        }
    }

    pub fn device_names(&self) -> &[String] {
        self.device_names.as_slice()
    }

    pub fn update_device_list(&mut self) -> anyhow::Result<()> {
        self.device_names = self.audio_receiver.list_devices()?.to_vec();
        Ok(())
    }

    pub fn audio_mut(&mut self) -> &mut Vec<Vec<f32>> {
        &mut self.audio_buffer
    }

    pub fn connect_to_audio_input(&mut self, device_index: usize) -> anyhow::Result<()> {
        let device_name = &self.device_names[device_index];
        self.audio_receiver.select_device(device_name)?;
        self.audio_receiver.open_stream()?;
        Ok(())
    }

    pub fn fetch_audio(&mut self) {
        while let Ok(mut channel_data) = self.audio_receiver.try_receive_audio() {
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
