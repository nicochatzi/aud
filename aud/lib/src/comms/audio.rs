use super::*;
use crate::audio::*;
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
    packets: AudioPacketSequence,
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
            packets: AudioPacketSequence::default(),
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
            log::error!("Failed to send GetDevices message : {e}");
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
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                AudioResponse::Devices(mut devices) => self.devices = std::mem::take(&mut devices),
                AudioResponse::Audio(packet) => {
                    self.has_connected = true;
                    self.packets.push(packet);
                }
            }
        }

        Ok(self.packets.drain())
    }
}

// Counterpart to `AudioReceiver`
//
// This takes an `AudioProvider`
// and managing transmitting
// its data over a socket.
pub struct AudioTransmitter<AudioProvider> {
    audio_provider: AudioProvider,
    requests: Receiver<AudioRequest>,
    responses: Sender<AudioResponse>,
    packet_count: u64,
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
            packet_count: 0,
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

        if let Err(e) = self
            .responses
            .try_send(AudioResponse::Audio(AudioPacket::new(
                self.packet_count,
                audio,
            )))
        {
            log::error!("Failed to pass audio response : {e}");
        }

        self.packet_count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::comms::test::{MockSocket, ADDR};
    use std::sync::{Arc, Mutex};

    #[test]
    fn audio_receiver_can_request_devices() {}

    #[test]
    fn audio_receiver_can_fetch_audio_buffers() {}

    #[test]
    fn audio_transmitter_can_send_device_list() {}

    #[test]
    fn audio_transmitter_can_send_audio() {}

    // #[test]
    // fn audio_receiver_can_reorder_packets_after_order_was_mangled() {
    //     const NUM_RESPONSES_TO_PARSE: usize = 16;
    //     let mut channel_count = 0;
    //     let mut packet_count = 4;
    //
    //     let respond_with_mangled_packet = move |buf: &mut [u8]| {
    //         channel_count += 1;
    //         packet_count -= 1;
    //         if packet_count < 0 {
    //             packet_count = 4 as i32;
    //         }
    //
    //         let packet = AudioResponse::Audio(AudioPacket::new(
    //             packet_count as u64,
    //             vec![vec![0., 0.]; channel_count as usize],
    //         ))
    //         .serialize()
    //         .unwrap();
    //
    //         buf[..packet.len()].copy_from_slice(&packet);
    //         Ok((packet.len(), ADDR))
    //     };
    //
    //     let packet_mangling_socket = MockSocket {
    //         on_recv: Some(Arc::new(Mutex::new(respond_with_mangled_packet))),
    //         on_send: None,
    //     };
    //
    //     let mut audio_recv = AudioReceiver::with_address(Sockets {
    //         socket: packet_mangling_socket,
    //         target: ADDR,
    //     })
    //     .unwrap();
    //
    //     let mut groups_of_received_audio_buffers = vec![];
    //
    //     while groups_of_received_audio_buffers.len() < NUM_RESPONSES_TO_PARSE {
    //         let buf = audio_recv.try_fetch_audio().unwrap();
    //         if buf.is_empty() {
    //             continue;
    //         }
    //         groups_of_received_audio_buffers.push(buf);
    //     }
    //
    //     for (i, audio_buffers) in groups_of_received_audio_buffers.iter().enumerate() {
    //         let channel_count = audio_buffers.len();
    //         assert_eq!(channel_count, i + 1);
    //     }
    // }
    //
    // #[test]
    // fn audio_receiver_will_repeat_a_packet_if_one_is_corrupt() {
    //     const NUM_RESPONSES_TO_PARSE: usize = 2;
    //     let mut packet_count = 0;
    //
    //     let respond_with_mangled_packet = move |buf: &mut [u8]| {
    //         let packet_payload = vec![vec![packet_count as f32; 2]; 1];
    //         let mut packet = AudioPacket::new(packet_count as u64, packet_payload);
    //
    //         // Mangle the second packet
    //         if packet_count == 1 {
    //             packet.metadata.checksum = packet.metadata.checksum.wrapping_add(100);
    //         }
    //
    //         let response = AudioResponse::Audio(packet).serialize().unwrap();
    //         packet_count += 1;
    //
    //         buf[..response.len()].copy_from_slice(&response);
    //         Ok((response.len(), ADDR))
    //     };
    //
    //     let packet_mangling_socket = MockSocket {
    //         on_recv: Some(Arc::new(Mutex::new(respond_with_mangled_packet))),
    //         on_send: None,
    //     };
    //
    //     let mut audio_recv = AudioReceiver::with_address(Sockets {
    //         socket: packet_mangling_socket,
    //         target: ADDR,
    //     })
    //     .unwrap();
    //
    //     let mut groups_of_received_audio_buffers = vec![];
    //
    //     while groups_of_received_audio_buffers.len() < NUM_RESPONSES_TO_PARSE {
    //         let buf = audio_recv.try_fetch_audio().unwrap();
    //         if !buf.is_empty() {
    //             groups_of_received_audio_buffers.push(buf);
    //         }
    //     }
    //
    //     // Check that the first packet's payload was correctly received
    //     assert_eq!(groups_of_received_audio_buffers[0][0][0], 0.0);
    //     assert_eq!(groups_of_received_audio_buffers[0][0][1], 0.0);
    //
    //     // Check that the second packet's payload matches the first (because it was mangled and replaced)
    //     assert_eq!(groups_of_received_audio_buffers[1][0][0], 0.0);
    //     assert_eq!(groups_of_received_audio_buffers[1][0][1], 0.0);
    // }
}
