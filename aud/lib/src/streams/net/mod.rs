use std::{io, net::SocketAddr};

mod api;
mod audio;
mod comms;

pub use api::*;
pub use audio::*;
pub use comms::*;

pub trait SocketInterface: Send {
    fn receive(&mut self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;
    fn transmit(&mut self, buf: &[u8]) -> io::Result<usize>;
}

impl SocketInterface for std::net::UdpSocket {
    fn receive(&mut self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.recv_from(buf)
    }

    fn transmit(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.send(buf)
    }
}
