use cpal::traits::*;
use crossbeam::channel::{Receiver, Sender};

pub struct App {
    device_names: Vec<String>,
    selection: Option<String>,
    audio_buffer: Vec<Vec<f32>>,
    sender: Sender<Vec<Vec<f32>>>,
    receiver: Receiver<Vec<Vec<f32>>>,
    host: cpal::Host,
    stream: Option<cpal::Stream>,
}

impl Default for App {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(100);
        let host = cpal::default_host();

        Self {
            device_names: vec![],
            selection: None,
            sender,
            receiver,
            host,
            stream: None,
            audio_buffer: vec![],
        }
    }
}

impl App {
    pub fn device_names(&self) -> &[String] {
        self.device_names.as_slice()
    }

    pub fn update_device_list(&mut self) -> anyhow::Result<()> {
        self.device_names = self
            .host
            .input_devices()?
            .map(|x| x.name().unwrap())
            .collect();

        Ok(())
    }

    pub fn audio_mut(&mut self) -> &mut Vec<Vec<f32>> {
        &mut self.audio_buffer
    }

    pub fn connect_to_audio_input(&mut self, device_index: usize) -> anyhow::Result<()> {
        let mut input_devices = self.host.input_devices()?;

        let Some(device) = input_devices.find(|x| {
            x.name()
                .map(|y| y == self.device_names[device_index])
                .unwrap_or(false)
        }) else {
            anyhow::bail!("");
        };

        if let Some(stream) = self.stream.take() {
            stream.pause()?;
        }

        self.selection = Some(device.name()?);
        self.stream = Some(crate::audio::stream(
            self.sender.clone(),
            &device,
            device.default_input_config()?,
            crate::audio::Direction::In,
        )?);

        Ok(())
    }

    pub fn fetch_audio(&mut self) {
        while let Ok(mut channel_data) = self.receiver.try_recv() {
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
