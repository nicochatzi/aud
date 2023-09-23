use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
};

mod api;
mod audio;
mod sockets;

pub use api::*;
pub use audio::*;
pub use sockets::*;

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

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use std::{
        io,
        net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs},
        sync::{Arc, Mutex},
    };

    pub const ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    #[derive(Default, Clone)]
    pub struct MockSocket {
        pub on_send: Option<Arc<Mutex<dyn FnMut(&[u8]) -> io::Result<usize>>>>,
        pub on_recv: Option<Arc<Mutex<dyn FnMut(&mut [u8]) -> io::Result<(usize, SocketAddr)>>>>,
    }

    unsafe impl Send for MockSocket {}
    unsafe impl Sync for MockSocket {}

    impl MockSocket {
        pub fn with_hooks<S, R>(on_send: S, on_recv: R) -> Self
        where
            S: FnMut(&[u8]) -> io::Result<usize> + 'static,
            R: FnMut(&mut [u8]) -> io::Result<(usize, SocketAddr)> + 'static,
        {
            Self {
                on_send: Some(Arc::new(Mutex::new(on_send))),
                on_recv: Some(Arc::new(Mutex::new(on_recv))),
            }
        }

        pub fn with_send_hook<S>(on_send: S) -> Self
        where
            S: FnMut(&[u8]) -> io::Result<usize> + 'static,
        {
            Self {
                on_send: Some(Arc::new(Mutex::new(on_send))),
                on_recv: None,
            }
        }

        pub fn with_recv_hook<R>(on_recv: R) -> Self
        where
            R: FnMut(&mut [u8]) -> io::Result<(usize, SocketAddr)> + 'static,
        {
            Self {
                on_send: None,
                on_recv: Some(Arc::new(Mutex::new(on_recv))),
            }
        }
    }

    impl SocketInterface for MockSocket {
        fn try_to_clone(&self) -> io::Result<Self> {
            Ok(Self {
                on_recv: self.on_recv.clone(),
                on_send: self.on_send.clone(),
            })
        }

        fn receive(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
            let Some(hook) = &self.on_recv else {
                return Ok((0, ADDR));
            };

            let mut hook_fn = hook
                .try_lock()
                .map_err(|_| io::Error::new(io::ErrorKind::WouldBlock, "Failed to lock"))?;

            hook_fn(buf)
        }

        fn transmit<T: ToSocketAddrs>(&self, buf: &[u8], _target: T) -> io::Result<usize> {
            let Some(hook) = &self.on_send else {
                return Ok(0);
            };

            let mut hook_fn = hook
                .try_lock()
                .map_err(|_| io::Error::new(io::ErrorKind::WouldBlock, "Failed to lock"))?;

            hook_fn(buf)
        }
    }
}
