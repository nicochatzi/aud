//! Additional functionality to add to the engine.
//!
//! Access it by including the traits you need.

use super::LuaRuntime;

pub mod hooks {
    use super::*;

    pub trait TraceHookProviding {
        fn on_start(&self) -> anyhow::Result<()>;
        fn on_stop(&self) -> anyhow::Result<()>;
    }

    pub trait ConnectionHookProviding {
        fn on_discover(&self, device_names: &[String]) -> anyhow::Result<()>;
        fn on_connect(&self, device_name: &str) -> anyhow::Result<()>;
    }

    pub trait MidiHookProviding {
        fn on_midi(&self, device_name: &str, bytes: &[u8]) -> anyhow::Result<Option<bool>>;
    }

    pub trait AudioHookProviding {
        fn on_audio(&self, device_name: &str, data: Vec<Vec<f32>>) -> anyhow::Result<()>;
    }

    impl TraceHookProviding for LuaRuntime {
        fn on_start(&self) -> anyhow::Result<()> {
            match self.has_script() {
                true => self.call("on_start", ()),
                false => Ok(()),
            }
        }

        fn on_stop(&self) -> anyhow::Result<()> {
            match self.has_script() {
                true => self.call("on_stop", ()),
                false => Ok(()),
            }
        }
    }

    impl ConnectionHookProviding for LuaRuntime {
        fn on_discover(&self, device_names: &[String]) -> anyhow::Result<()> {
            match self.has_script() {
                true => self.call("on_discover", device_names),
                false => Ok(()),
            }
        }

        fn on_connect(&self, device_name: &str) -> anyhow::Result<()> {
            match self.has_script() {
                true => self.call("on_connect", device_name),
                false => Ok(()),
            }
        }
    }

    impl MidiHookProviding for LuaRuntime {
        fn on_midi(&self, device_name: &str, bytes: &[u8]) -> anyhow::Result<Option<bool>> {
            match self.has_script() {
                true => self.call("on_midi", (device_name, bytes)),
                false => Ok(None),
            }
        }
    }

    impl AudioHookProviding for LuaRuntime {
        fn on_audio(&self, device_name: &str, data: Vec<Vec<f32>>) -> anyhow::Result<()> {
            match self.has_script() {
                true => self.call("on_audio", (device_name, data)),
                false => Ok(()),
            }
        }
    }
}

pub mod api {
    use super::*;
    use crossbeam::channel::Sender;

    pub enum LogApiEvent {
        Log(String),
        Alert(String),
    }

    pub trait LogProviding<E>
    where
        E: From<LogApiEvent>,
    {
        fn load_log(&self, name: String, tx: Sender<E>) -> anyhow::Result<()>;
        fn load_alert(&self, name: String, tx: Sender<E>) -> anyhow::Result<()>;
    }

    pub struct ConnectionApiEvent {
        pub device: String,
    }

    pub trait ConnectionProviding<E>
    where
        E: From<ConnectionApiEvent>,
    {
        fn load_connect(&self, name: String, tx: Sender<E>) -> anyhow::Result<()>;
    }

    pub enum ControlFlowApiEvent {
        Pause,
        Resume,
        Stop,
    }

    pub trait ControlFlowProviding<E>
    where
        E: From<ControlFlowApiEvent>,
    {
        fn load_pause(&self, name: String, tx: Sender<E>) -> anyhow::Result<()>;
        fn load_resume(&self, name: String, tx: Sender<E>) -> anyhow::Result<()>;
        fn load_stop(&self, name: String, tx: Sender<E>) -> anyhow::Result<()>;
    }

    impl<E> LogProviding<E> for LuaRuntime
    where
        E: From<LogApiEvent> + 'static,
    {
        fn load_log(&self, name: String, tx: Sender<E>) -> anyhow::Result<()> {
            self.set_fn("log", {
                move |_, message: String| {
                    if let Err(e) = tx.try_send(LogApiEvent::Log(message).into()) {
                        log::error!("{name} ! failed to send log event : {}", e);
                    }
                    Ok(())
                }
            })
        }

        fn load_alert(&self, name: String, tx: Sender<E>) -> anyhow::Result<()> {
            self.set_fn("alert", {
                move |_, message: String| {
                    if let Err(e) = tx.try_send(LogApiEvent::Alert(message).into()) {
                        log::error!("{name} ! failed to send alert event : {}", e);
                    }
                    Ok(())
                }
            })
        }
    }

    impl<E> ConnectionProviding<E> for LuaRuntime
    where
        E: From<ConnectionApiEvent> + 'static,
    {
        fn load_connect(&self, name: String, tx: Sender<E>) -> anyhow::Result<()> {
            self.set_fn("connect", {
                move |_, device: String| {
                    if let Err(e) = tx.try_send(ConnectionApiEvent { device }.into()) {
                        log::error!("{name} ! failed to send connection event : {}", e);
                    }
                    Ok(())
                }
            })
        }
    }

    impl<E> ControlFlowProviding<E> for LuaRuntime
    where
        E: From<ControlFlowApiEvent> + 'static,
    {
        fn load_pause(&self, name: String, tx: Sender<E>) -> anyhow::Result<()> {
            self.set_fn("pause", {
                move |_, (): ()| {
                    if let Err(e) = tx.try_send(ControlFlowApiEvent::Pause.into()) {
                        log::error!("{name} ! failed to send pause event : {}", e);
                    }
                    Ok(())
                }
            })
        }

        fn load_resume(&self, name: String, tx: Sender<E>) -> anyhow::Result<()> {
            self.set_fn("resume", {
                move |_, (): ()| {
                    if let Err(e) = tx.try_send(ControlFlowApiEvent::Resume.into()) {
                        log::error!("{name} ! failed to send resume event : {}", e);
                    }
                    Ok(())
                }
            })
        }

        fn load_stop(&self, name: String, tx: Sender<E>) -> anyhow::Result<()> {
            self.set_fn("stop", {
                move |_, (): ()| {
                    if let Err(e) = tx.try_send(ControlFlowApiEvent::Stop.into()) {
                        log::error!("{name} ! failed to send stop event : {}", e);
                    }
                    Ok(())
                }
            })
        }
    }
}
