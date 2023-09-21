use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
};

mod api;
mod audio;
mod comms;

pub use api::*;
pub use audio::*;
pub use comms::*;

pub trait SocketInterface: Send + Sized {
    fn receive(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;
    fn transmit<T: ToSocketAddrs>(&self, buf: &[u8], target: T) -> io::Result<usize>;
    fn try_to_clone(&self) -> io::Result<Self>;
}

impl SocketInterface for std::net::UdpSocket {
    fn receive(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.recv_from(buf)
    }

    fn transmit<T: ToSocketAddrs>(&self, buf: &[u8], target: T) -> io::Result<usize> {
        self.send_to(buf, target)
    }

    fn try_to_clone(&self) -> io::Result<Self> {
        self.try_clone()
    }
}
