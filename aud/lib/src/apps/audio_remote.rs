use crossbeam::channel;

use crate::{
    audio::{
        AudioBuffer, AudioChannelSelection, AudioConsuming, AudioDevice, AudioDeviceConnection,
        AudioInterface, AudioProviding, RemoteAudioReceiver,
    },
    comms::{SocketInterface, Sockets},
};

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
