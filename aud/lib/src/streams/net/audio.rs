use super::*;
use crate::streams::audio::*;
use crossbeam::channel::{Receiver, Sender};

/// `AudioProviding` struct that acts as a proxy
/// to a remote `AudioProviding` struct.
///
/// This struct will resend requests and parse
/// responses over some socket connection, e.g. UDP.
///
/// It should be paired with an `net::AudioTransmitter`
///
pub struct AudioReceiver {
    devices: Vec<AudioDevice>,
    sender: Sender<AudioRequest>,
    receiver: Receiver<AudioResponse>,
    has_connected: bool,
    _handle: SocketCommunicator,
}

impl AudioReceiver {
    pub fn with_address<Socket>(sockets: Sockets<Socket>) -> anyhow::Result<Self>
    where
        Socket: SocketInterface + 'static,
    {
        let (request_tx, request_rx) = crossbeam::channel::bounded(8);
        let (response_tx, response_rx) = crossbeam::channel::bounded(16);

        Ok(Self {
            devices: vec![],
            sender: request_tx,
            receiver: response_rx,
            has_connected: false,
            _handle: SocketCommunicator::launch(
                sockets,
                Events {
                    inputs: request_rx,
                    outputs: response_tx,
                },
            ),
        })
    }
}

impl AudioProviding for AudioReceiver {
    fn is_connected(&self) -> bool {
        self.has_connected
    }

    fn list_audio_devices(&self) -> &[AudioDevice] {
        if let Err(e) = self.sender.send(AudioRequest::GetDevices) {
            log::error!("Failed to send GetDevices message");
        }
        self.devices.as_slice()
    }

    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        self.has_connected = false;
        self.sender.send(AudioRequest::Connect {
            device: audio_device.clone(),
            channels: channel_selection.clone(),
        })?;
        Ok(())
    }

    fn try_fetch_audio(&mut self) -> anyhow::Result<AudioBuffer> {
        let mut audio = vec![];

        while let Ok(event) = self.receiver.try_recv() {
            match event {
                AudioResponse::Devices(mut devices) => self.devices = std::mem::take(&mut devices),
                AudioResponse::Audio(mut buffers) => {
                    self.has_connected = true;
                    audio.append(&mut buffers)
                }
            }
        }

        Ok(audio)
    }
}

/// Counterpart to `AudioReceiver`
///
/// This takes an `AudioProvider`
/// and managing transmitting
/// its data over a socket.
pub struct AudioTransmitter<AudioProvider: AudioProviding> {
    audio_provider: AudioProvider,
    requests: Receiver<AudioRequest>,
    responses: Sender<AudioResponse>,
    _handle: SocketCommunicator,
}

impl<AudioProvider: AudioProviding> AudioTransmitter<AudioProvider> {
    pub fn new<Socket>(
        sockets: Sockets<Socket>,
        audio_provider: AudioProvider,
    ) -> anyhow::Result<Self>
    where
        Socket: SocketInterface + 'static,
    {
        let (response_tx, response_rx) = crossbeam::channel::bounded::<AudioResponse>(16);
        let (request_tx, request_rx) = crossbeam::channel::bounded::<AudioRequest>(8);

        Ok(Self {
            audio_provider,
            requests: request_rx,
            responses: response_tx,
            _handle: SocketCommunicator::launch(
                sockets,
                Events {
                    inputs: response_rx,
                    outputs: request_tx,
                },
            ),
        })
    }

    pub fn is_audio_connected(&self) -> bool {
        self.audio_provider.is_connected()
    }

    pub fn process_requests(&mut self) -> anyhow::Result<()> {
        while let Ok(request) = self.requests.try_recv() {
            match request {
                AudioRequest::GetDevices => self.responses.try_send(AudioResponse::Devices(
                    self.audio_provider.list_audio_devices().to_vec(),
                ))?,
                AudioRequest::Connect { device, channels } => self
                    .audio_provider
                    .connect_to_audio_device(&device, channels)?,
            }
        }

        Ok(())
    }

    pub fn try_send_audio(&mut self) -> anyhow::Result<()> {
        let audio = self.audio_provider.try_fetch_audio()?;

        if audio.is_empty() {
            return Ok(());
        }

        if let Err(e) = self.responses.try_send(AudioResponse::Audio(audio)) {
            log::error!("Failed to pass audio response : {e}");
        }

        Ok(())
    }
}
