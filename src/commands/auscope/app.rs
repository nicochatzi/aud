use crate::widgets::StatefulList;
use cpal::traits::*;
use crossbeam::channel::{Receiver, Sender};
use crossterm::event::KeyCode;

pub struct App {
    pub device_names: StatefulList<String>,
    pub selection: Option<String>,
    pub is_running: bool,
    pub show_usage: bool,
    pub audio_buffer: Vec<Vec<f32>>,

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
            device_names: StatefulList::default(),
            selection: None,
            sender,
            receiver,
            host,
            stream: None,
            audio_buffer: vec![],
            is_running: true,
            show_usage: false,
        }
    }
}

impl App {
    pub fn update_device_list(&mut self) -> anyhow::Result<()> {
        self.device_names = StatefulList::with_items(
            self.host
                .input_devices()?
                .map(|x| x.name().unwrap())
                .collect(),
        );

        Ok(())
    }

    pub fn connect(&mut self) -> anyhow::Result<()> {
        let mut input_devices = self.host.input_devices()?;
        let Some(selected_index) = self.device_names.selected() else {
            anyhow::bail!("");
        };

        let Some(device) = input_devices.find(|x| {
            x.name()
                .map(|y| y == self.device_names.items[selected_index])
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
            crate::audio::Dir::In,
        )?);

        Ok(())
    }
}

impl crate::app::Base for App {
    fn setup(&mut self) -> anyhow::Result<()> {
        self.update_device_list()
    }

    fn update(&mut self) -> anyhow::Result<crate::app::Flow> {
        if !self.is_running {
            return Ok(crate::app::Flow::Loop);
        }

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

        Ok(crate::app::Flow::Continue)
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<crate::app::Flow> {
        match key.code {
            KeyCode::Char('?') => self.show_usage = !self.show_usage,
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.show_usage {
                    self.show_usage = false;
                } else {
                    return Ok(crate::app::Flow::Exit);
                }
            }
            KeyCode::Char(' ') => self.is_running = !self.is_running,
            KeyCode::Down | KeyCode::Char('j') => self.device_names.next(),
            KeyCode::Up | KeyCode::Char('k') => self.device_names.previous(),
            KeyCode::Enter => {
                if self.device_names.selected().is_some() {
                    self.device_names.confirm_selection();
                    self.is_running = true;
                    for buf in &mut self.audio_buffer {
                        buf.clear();
                    }
                    self.connect()?;
                }
            }
            _ => {}
        }

        Ok(crate::app::Flow::Continue)
    }
}
