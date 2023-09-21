use std::net::SocketAddr;

use super::{api::*, SocketInterface};

pub struct Sockets<Socket>
where
    Socket: SocketInterface,
{
    pub socket: Socket,
    pub target: SocketAddr,
}

pub struct Events<InputEvent, OutputEvent>
where
    InputEvent: BincodeSerialize,
    OutputEvent: BincodeDeserialize,
{
    pub inputs: crossbeam::channel::Receiver<InputEvent>,
    pub outputs: crossbeam::channel::Sender<OutputEvent>,
}

/// This is a black-box socket communicator.
/// It takes inputs events, transmits them over the transmitter socket.
/// It receives data from the receiver socket and pushes the output event.
///
/// Think of it as a black box in which you push
/// input events and receive output events.
///
/// Under the hood it starts dedicated threads
/// for both tasks.
pub struct SocketCommunicator {
    request_handle: Option<std::thread::JoinHandle<()>>,
    response_handle: Option<std::thread::JoinHandle<()>>,
    shutdown_sender: crossbeam::channel::Sender<()>,
}

impl SocketCommunicator {
    pub fn launch<Socket, InputEvent, OutputEvent>(
        sockets: Sockets<Socket>,
        events: Events<InputEvent, OutputEvent>,
    ) -> Self
    where
        Socket: SocketInterface + 'static,
        InputEvent: BincodeSerialize + Send + 'static,
        OutputEvent: BincodeDeserialize + Send + 'static,
    {
        let Events { inputs, outputs } = events;
        let Sockets { socket, target } = sockets;
        let socket_tx = socket.try_to_clone().unwrap();

        let (shutdown_sender, shutdown_receiver) = crossbeam::channel::bounded::<()>(2);

        let shutdown_receiver_clone = shutdown_receiver.clone();
        let audio_request_handle = std::thread::spawn(move || {
            let shutdown_receiver = shutdown_receiver_clone.clone();

            loop {
                crossbeam::select! {
                    recv(inputs) -> event => handle_input_event(&socket_tx, &target, event),
                    recv(shutdown_receiver) -> _ => {
                        log::trace!("socket transmitter shutting down");
                        return;
                    },
                }
            }
        });

        let shutdown_receiver_clone = shutdown_receiver.clone();
        let udp_response_handle = std::thread::spawn(move || {
            let shutdown_receiver = shutdown_receiver_clone.clone();
            let mut udp_buffer = vec![0u8; 4096];

            loop {
                crossbeam::select! {
                    default => parse_socket_buffer(&socket, &mut udp_buffer, &outputs),
                    recv(shutdown_receiver) -> _ => {
                        log::trace!("socket receiver shutting down");
                        return;
                    },
                }
            }
        });

        Self {
            request_handle: Some(audio_request_handle),
            response_handle: Some(udp_response_handle),
            shutdown_sender,
        }
    }

    fn shutdown_thread(&self, handle: std::thread::JoinHandle<()>) {
        if handle.join().is_err() {
            log::error!("Failed to join socket background task thread");
        }
    }
}

impl Drop for SocketCommunicator {
    fn drop(&mut self) {
        // send two shutdowns, one for each background thread
        if let Err(e) = self
            .shutdown_sender
            .try_send(())
            .or(self.shutdown_sender.try_send(()))
        {
            log::error!("Failed to send shutdown signal to background task : {e}");
        }

        if let Some(handle) = self.request_handle.take() {
            self.shutdown_thread(handle);
        }

        if let Some(handle) = self.response_handle.take() {
            self.shutdown_thread(handle);
        }
    }
}

fn handle_input_event<Socket, InputEvent>(
    socket: &Socket,
    target: &SocketAddr,
    request: Result<InputEvent, crossbeam::channel::RecvError>,
) where
    Socket: SocketInterface + 'static,
    InputEvent: BincodeSerialize + Send + 'static,
{
    let Ok(request) = request else {
        return;
    };

    let Ok(request) = request.serialize() else {
        log::error!("Failed to serialize request");
        return;
    };

    if let Err(e) = socket.transmit(&request, target) {
        log::error!("Failed to send request: {:?}", e);
    }

    log::trace!("Serialised and transmitted");
}

fn parse_socket_buffer<Socket, OutputEvent>(
    socket: &Socket,
    udp_buffer: &mut [u8],
    responses: &crossbeam::channel::Sender<OutputEvent>,
) where
    Socket: SocketInterface,
    OutputEvent: BincodeDeserialize,
{
    match socket.receive(udp_buffer) {
        Ok((size, _src)) => transmit_output_event(&udp_buffer[..size], responses),
        Err(e) => log::error!("socket reception error: {:?}", e),
    }
}

fn transmit_output_event<OutputEvent>(
    buffer: &[u8],
    responses: &crossbeam::channel::Sender<OutputEvent>,
) where
    OutputEvent: BincodeDeserialize,
{
    let Ok(response) = OutputEvent::deserialized(buffer) else {
        return;
    };

    if let Err(e) = responses.send(response) {
        log::error!("Failed to send response: {:?}", e);
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use std::{
//         io,
//         net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs},
//     };

//     const ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

//     #[derive(Default, Clone)]
//     struct MockSocket {
//         on_send: Option<Box<dyn Fn(&[u8]) -> io::Result<usize>>>,
//         on_recv: Option<Box<dyn Fn(&mut [u8]) -> io::Result<(usize, SocketAddr)>>>,
//     }

//     unsafe impl Send for MockSocket {}
//     unsafe impl Sync for MockSocket {}

//     impl SocketInterface for MockSocket {
//         fn try_to_clone(&self) -> io::Result<Self> {
//             Ok(Self {
//                 on_recv: self.on_recv,
//                 on_send: self.on_send,
//             })
//         }

//         fn receive(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
//             let Some(hook) = self.on_recv.as_mut() else {
//                 return Ok((0, ADDR));
//             };

//             hook(buf)
//         }

//         fn transmit<T: ToSocketAddrs>(&self, buf: &[u8], _target: T) -> io::Result<usize> {
//             let Some(hook) = self.on_send.as_mut() else {
//                 return Ok(0);
//             };

//             hook(buf)
//         }
//     }

//     #[test]
//     fn udp_communicator_terminates_background_tasks_when_dropped() {
//         let (_, requests) = crossbeam::channel::unbounded::<AudioRequest>();
//         let (events, _) = crossbeam::channel::unbounded::<AudioResponse>();
//         {
//             let _ = SocketCommunicator::launch(
//                 Sockets {
//                     socket: MockSocket::default(),
//                     target: ADDR,
//                 },
//                 Events {
//                     inputs: requests,
//                     outputs: events,
//                 },
//             );
//         }
//     }

//     // #[test]
//     // fn udp_communicator_can_parse_udp_responses_and_send_them_back_to_an_output_channel() {
//     //     let (request_tx, request_rx) = crossbeam::channel::unbounded::<AudioRequest>();
//     //     let (response_tx, response_rx) = crossbeam::channel::unbounded::<AudioResponse>();

//     //     let expected_response = AudioResponse::Audio(vec![vec![1., 2.]; 2]);

//     //     let udp_receiver = MockSocket {
//     //         on_recv: Some(Box::new({
//     //             let expected_response = expected_response.clone();

//     //             move |buf| {
//     //                 let response = expected_response.clone().serialize().unwrap();
//     //                 buf[..response.len()].copy_from_slice(&response);
//     //                 Ok((response.len(), ADDR))
//     //             }
//     //         })),
//     //         ..Default::default()
//     //     };

//     //     let _comms = SocketCommunicator::launch(
//     //         Sockets {
//     //             socket: MockSocket::default(),
//     //             target: udp_receiver,
//     //         },
//     //         Events {
//     //             inputs: request_rx,
//     //             outputs: response_tx,
//     //         },
//     //     );

//     //     let response = response_rx
//     //         .recv_timeout(std::time::Duration::from_secs(1))
//     //         .unwrap();

//     //     assert_eq!(response, expected_response);
//     // }

//     #[test]
//     fn udp_communicator_can_send_requests_over_udp_given_a_push_to_an_input_channel() {
//         let (request_tx, request_rx) = crossbeam::channel::unbounded::<AudioRequest>();
//         let (response_tx, response_rx) = crossbeam::channel::unbounded::<AudioResponse>();
//         let (on_send_tx, on_send_rx) = crossbeam::channel::unbounded();

//         let expected_request = AudioRequest::GetDevices;
//         let expected_response = AudioResponse::Audio(vec![vec![1., 2.]; 2]);

//         let udp_transmitter = MockSocket {
//             on_send: Some(Box::new({
//                 let expected_request = expected_request.clone();
//                 let expected_response = expected_response.clone();

//                 move |buf| {
//                     let request = AudioRequest::deserialized(buf).unwrap();
//                     assert_eq!(expected_request, request);
//                     on_send_tx.send(expected_response.clone()).unwrap();
//                     Ok(0)
//                 }
//             })),
//             ..Default::default()
//         };

//         let _comms = SocketCommunicator::launch(
//             Sockets {
//                 socket: udp_transmitter,
//                 target: ADDR,
//             },
//             Events {
//                 inputs: request_rx,
//                 outputs: response_tx,
//             },
//         );

//         request_tx
//             .send_timeout(expected_request, std::time::Duration::from_secs(1))
//             .unwrap();

//         let response = on_send_rx
//             .recv_timeout(std::time::Duration::from_secs(1))
//             .unwrap();

//         assert_eq!(response, expected_response);
//     }
// }
