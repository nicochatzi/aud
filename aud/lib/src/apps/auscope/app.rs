use crate::{
    audio::*,
    comms::{SocketInterface, Sockets},
};
use crossbeam::channel;

pub trait AudioProvider: AudioProviding + AudioInterface {}

impl<T> AudioProvider for T where T: AudioProviding + AudioInterface {}

pub struct App {
    buffer: AudioBuffer,
    receiver: Box<dyn AudioProvider>,
    device: Option<AudioDevice>,
    num_received: usize,
}

impl App {
    pub fn new(audio_receiver: Box<dyn AudioProvider>) -> Self {
        Self {
            buffer: AudioBuffer::default(),
            receiver: audio_receiver,
            device: None,
            num_received: 0,
        }
    }

    pub fn devices(&self) -> &[AudioDevice] {
        self.receiver.list_audio_devices()
    }

    pub fn audio_mut(&mut self) -> &mut AudioBuffer {
        &mut self.buffer
    }

    pub fn update_channel_selection(
        &mut self,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        let Some(ref audio_device) = self.device else {
            anyhow::bail!("No audio device selected");
        };

        self.buffer.data.clear();
        self.receiver
            .connect_to_audio_device(audio_device, channel_selection)?;
        self.device = Some(audio_device.clone());
        Ok(())
    }

    pub fn connect_to_audio_input(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        self.device = Some(audio_device.clone());
        self.buffer.num_channels = channel_selection.count() as u32;
        self.update_channel_selection(channel_selection)
    }

    pub fn fetch_audio(&mut self) -> anyhow::Result<()> {
        self.receiver.process_audio_events()?;

        let mut audio = self.receiver.retrieve_audio_buffer();

        if !audio.data.is_empty() {
            log::info!("num received {}", self.num_received);
            self.num_received += 1;
        }

        if self.buffer.num_channels != audio.num_channels {
            self.buffer = audio;
        } else {
            self.buffer.data.append(&mut audio.data);
        }

        Ok(())
    }
}

// Pipes audio received from the remote into the provider.
// `RemoteAudioReceiver` receives audio and pushes into a
// consumer. In our case, we want to grab that audio
// through the `RemteAudioProvider`.
struct AudioPipe {
    sender: channel::Sender<AudioBuffer>,
}

impl AudioConsuming for AudioPipe {
    fn consume_audio_buffer(&mut self, buffer: AudioBuffer) -> anyhow::Result<()> {
        self.sender.try_send(buffer)?;
        Ok(())
    }
}

pub struct RemoteAudioProvider {
    interface: RemoteAudioReceiver<AudioPipe>,
    receiver: channel::Receiver<AudioBuffer>,
}

impl RemoteAudioProvider {
    pub fn new<Socket>(sockets: Sockets<Socket>) -> anyhow::Result<Self>
    where
        Socket: SocketInterface + 'static,
    {
        let (sender, receiver) = channel::bounded(16);
        let pipe = AudioPipe { sender };
        let interface = RemoteAudioReceiver::new(pipe, sockets)?;

        Ok(Self {
            interface,
            receiver,
        })
    }
}

impl AudioProviding for RemoteAudioProvider {
    fn retrieve_audio_buffer(&mut self) -> AudioBuffer {
        self.receiver.try_recv().unwrap_or_default()
    }
}

impl AudioInterface for RemoteAudioProvider {
    fn is_accessible(&self) -> bool {
        self.interface.is_accessible()
    }

    fn list_audio_devices(&self) -> &[AudioDevice] {
        self.interface.list_audio_devices()
    }

    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        self.interface
            .connect_to_audio_device(audio_device, channel_selection)
    }

    fn connected_audio_device(&self) -> Option<&AudioDeviceConnection> {
        self.interface.connected_audio_device()
    }

    fn process_audio_events(&mut self) -> anyhow::Result<()> {
        self.interface.process_audio_events()
    }
}
