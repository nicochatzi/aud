use super::*;
use crate::comms::*;
use crossbeam::channel::{Receiver, Sender};

/// `RemoteAudioReceiver` acts as a facade to a remote `AudioProviding` struct,
/// proxying the audio data to the local `AudioConsumer`.
///
/// It takes an `AudioConsumer` instance and manages the flow of audio buffers from
/// the remote provider to the local consumer, handling the necessary communication
/// and synchronization.
pub struct RemoteAudioReceiver<AudioConsumer: AudioConsuming> {
    devices: Vec<AudioDevice>,
    sender: Sender<AudioRequest>,
    receiver: Receiver<AudioResponse>,
    is_remote_accessible: bool,
    packets: AudioPacketSequence,
    audio_consumer: AudioConsumer,
    connected_device: Option<AudioDeviceConnection>,
    _handle: SocketCommunicator,
}

impl<AudioConsumer: AudioConsuming> RemoteAudioReceiver<AudioConsumer> {
    pub fn new<Socket>(
        audio_consumer: AudioConsumer,
        sockets: Sockets<Socket>,
    ) -> anyhow::Result<Self>
    where
        Socket: SocketInterface + 'static,
    {
        let (request_tx, request_rx) = crossbeam::channel::bounded(8);
        let (response_tx, response_rx) = crossbeam::channel::bounded(16);

        Ok(Self {
            devices: vec![],
            sender: request_tx,
            receiver: response_rx,
            is_remote_accessible: false,
            audio_consumer,
            packets: AudioPacketSequence::default(),
            connected_device: None,
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

impl<AudioConsumer: AudioConsuming> AudioInterface for RemoteAudioReceiver<AudioConsumer> {
    fn is_accessible(&self) -> bool {
        self.is_remote_accessible
    }

    fn list_audio_devices(&self) -> &[AudioDevice] {
        if let Err(e) = self.sender.send(AudioRequest::GetDevices) {
            log::error!("Failed to pass GetDevices request to socket handler : {e}");
        }

        self.devices.as_slice()
    }

    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        self.is_remote_accessible = false;
        self.sender.send(AudioRequest::Connect {
            device: audio_device.clone(),
            channels: channel_selection.clone(),
        })?;
        Ok(())
    }

    fn connected_audio_device(&self) -> Option<&AudioDeviceConnection> {
        self.connected_device.as_ref()
    }

    fn process_audio_events(&mut self) -> anyhow::Result<()> {
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                AudioResponse::Connected(dev) => {
                    self.is_remote_accessible = true;
                    self.connected_device = Some(dev)
                }
                AudioResponse::Devices(mut devices) => {
                    self.is_remote_accessible = true;
                    self.devices = std::mem::take(&mut devices)
                }
                AudioResponse::Audio(packet) => {
                    self.is_remote_accessible = true;
                    self.packets.push(packet);
                }
            }
        }

        if self.packets.num_available_frames() != 0 {
            let buffer = AudioBuffer::from_buffers(self.packets.extract());
            self.audio_consumer.consume_audio_buffer(buffer)?;
        }

        Ok(())
    }
}

/// `RemoteAudioTransmitter` acts as a facade to a remote `AudioConsuming` struct,
/// proxying the audio data from the local `AudioProviding` to the remote consumer.
///
/// It takes an `AudioProviding` instance and manages the flow of audio buffers from
/// the local provider to the remote consumer, handling the necessary communication
/// and synchronization.
pub struct RemoteAudioTransmitter<AudioProvider> {
    audio_provider: AudioProvider,
    requests: Receiver<AudioRequest>,
    responses: Sender<AudioResponse>,
    sequence: AudioPacketSequenceBuilder,
    connected_device: Option<AudioDeviceConnection>,
    _handle: SocketCommunicator,
}

impl<AudioProvider> RemoteAudioTransmitter<AudioProvider>
where
    AudioProvider: AudioProviding + AudioInterface,
{
    pub fn new<Socket>(
        audio_provider: AudioProvider,
        sockets: Sockets<Socket>,
    ) -> anyhow::Result<Self>
    where
        Socket: SocketInterface + 'static,
    {
        let (response_tx, response_rx) = crossbeam::channel::bounded::<AudioResponse>(128);
        let (request_tx, request_rx) = crossbeam::channel::bounded::<AudioRequest>(8);

        Ok(Self {
            audio_provider,
            requests: request_rx,
            responses: response_tx,
            sequence: AudioPacketSequenceBuilder::default(),
            connected_device: None,
            _handle: SocketCommunicator::launch(
                sockets,
                Events {
                    inputs: response_rx,
                    outputs: request_tx,
                },
            ),
        })
    }

    fn purge_audio_cache(&mut self) {
        let _ = self.audio_provider.retrieve_audio_buffer();
    }

    fn try_send_audio(&mut self) {
        let buffer = self.audio_provider.retrieve_audio_buffer();
        for packet in self.sequence.from_buffer(&buffer).into_packets() {
            if let Err(e) = self.responses.try_send(AudioResponse::Audio(packet)) {
                log::error!("Failed to pass audio response to socket tasks : {e}");
            }
        }
    }

    fn process_socket_requests(&mut self) -> anyhow::Result<()> {
        while let Ok(request) = self.requests.try_recv() {
            match request {
                AudioRequest::GetDevices => {
                    let devices = self.audio_provider.list_audio_devices().to_vec();
                    self.responses.try_send(AudioResponse::Devices(devices))?;
                }
                AudioRequest::Connect { device, channels } => {
                    self.connect_to_audio_device(&device, channels.clone())?;
                }
            }
        }
        Ok(())
    }

    fn update_selection_from_provider(&mut self) {
        let should_update_device_info =
            self.connected_device.as_ref() != self.audio_provider.connected_audio_device();

        if should_update_device_info {
            self.connected_device = self.audio_provider.connected_audio_device().cloned();
            self.send_connected_response();
        }
    }

    fn send_connected_response(&self) {
        let Some(ref dev) = self.connected_device else {
            return;
        };

        if let Err(e) = self
            .responses
            .try_send(AudioResponse::Connected(dev.clone()))
        {
            log::error!("Failed to pass connected message to socket tasks : {e}");
        }
    }
}

impl<AudioProvider> AudioInterface for RemoteAudioTransmitter<AudioProvider>
where
    AudioProvider: AudioProviding + AudioInterface,
{
    fn is_accessible(&self) -> bool {
        self.audio_provider.is_accessible()
    }

    fn list_audio_devices(&self) -> &[AudioDevice] {
        self.audio_provider.list_audio_devices()
    }

    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        self.purge_audio_cache();
        self.audio_provider
            .connect_to_audio_device(audio_device, channel_selection)
    }

    fn connected_audio_device(&self) -> Option<&AudioDeviceConnection> {
        self.connected_device.as_ref()
    }

    fn process_audio_events(&mut self) -> anyhow::Result<()> {
        self.update_selection_from_provider();
        self.process_socket_requests()?;
        self.audio_provider.process_audio_events()?;
        self.try_send_audio();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::comms::test::{MockSocket, ADDR};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MockConsumer {
        on_consume: Option<Box<dyn FnMut(AudioBuffer) -> anyhow::Result<()>>>,
    }

    impl AudioConsuming for MockConsumer {
        fn consume_audio_buffer(&mut self, buffer: AudioBuffer) -> anyhow::Result<()> {
            let Some(hook) = self.on_consume.as_mut() else {
                return Ok(());
            };

            hook(buffer)
        }
    }

    #[test]
    fn receiver_can_request_devices() {
        let expected_device_list = vec![
            AudioDevice {
                name: "a".to_owned(),
                num_channels: 1,
            },
            AudioDevice {
                name: "b".to_owned(),
                num_channels: 2,
            },
            AudioDevice {
                name: "c".to_owned(),
                num_channels: 3,
            },
        ];

        let num_requests = Arc::new(Mutex::new(0));
        let hook_expecting_device_request = {
            let num_requests = num_requests.clone();

            move |buf: &[u8]| {
                assert_eq!(
                    AudioRequest::GetDevices,
                    AudioRequest::deserialized(buf).unwrap()
                );
                *num_requests.lock().unwrap() += 1;
                Ok(0)
            }
        };

        let hook_sending_device_list = {
            let device_list = Arc::new(expected_device_list.clone());
            let num_requests = num_requests.clone();

            move |buf: &mut [u8]| {
                if *num_requests.lock().unwrap() == 0 {
                    return Ok((0, ADDR));
                }

                let response = AudioResponse::Devices(device_list.to_vec())
                    .serialize()
                    .unwrap();
                buf[..response.len()].copy_from_slice(&response);
                Ok((response.len(), ADDR))
            }
        };

        let packet_mangling_socket =
            MockSocket::with_hooks(hook_expecting_device_request, hook_sending_device_list);

        let mut audio_recv = RemoteAudioReceiver::new(
            MockConsumer::default(),
            Sockets {
                socket: packet_mangling_socket,
                target: ADDR,
            },
        )
        .unwrap();

        assert!(audio_recv.list_audio_devices().is_empty());
        audio_recv.process_audio_events().unwrap();

        let mut timeout = 100;
        while !audio_recv.is_accessible() && timeout != 0 {
            audio_recv.process_audio_events().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
            timeout -= 1;
        }

        assert!(
            timeout > 0,
            "timed out before mock socket comms were accessible"
        );
        assert_eq!(*num_requests.lock().unwrap(), 1);
        assert_eq!(audio_recv.list_audio_devices(), expected_device_list);
    }

    #[test]
    fn receiver_can_fetch_audio_buffers() {}

    #[test]
    fn transmitter_can_send_device_list() {}

    #[test]
    fn transmitter_can_send_audio() {}
}
