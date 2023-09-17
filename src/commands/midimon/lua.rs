use crate::lua::LuaEngine;
use crossbeam::channel::{Receiver, Sender};

pub const DOCS: &str = include_str!("../../../api/midimon/docs.lua");
pub const API: &str = include_str!("../../../api/midimon/api.lua");

pub struct MidiData {
    pub timestamp: u64,
    pub bytes: Vec<u8>,
}

pub enum HostEvent {
    LoadScript { name: String, chunk: String },
    Discover(Vec<String>),
    Connect(String),
    Midi(MidiData),
    Stop,
    Terminate,
}

pub enum ScriptEvent {
    Log(String),
    Midi(MidiData),
    Connect(String),
    Alert(String),
    Pause,
    Resume,
    Stop,
}

pub struct LuaRuntime {
    ctx: LuaEngine,
    tx: Sender<ScriptEvent>,
    rx: Receiver<HostEvent>,
    device_name: Option<String>,
}

pub struct LuaRuntimeHandle {
    pub handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>,
    _tx: Sender<ScriptEvent>,
    _rx: Receiver<HostEvent>,
}

impl LuaRuntime {
    pub fn start(rx: Receiver<HostEvent>, tx: Sender<ScriptEvent>) -> LuaRuntimeHandle {
        let rx_ = rx.clone();
        let tx_ = tx.clone();
        let handle = std::thread::spawn(move || {
            let mut runtime = Self {
                ctx: crate::lua::LuaEngine::default(),
                rx: rx_,
                tx: tx_,
                device_name: None,
            };

            runtime.setup().unwrap();
            runtime.run().unwrap();
            Ok(())
        });

        LuaRuntimeHandle {
            handle: Some(handle),
            _tx: tx,
            _rx: rx,
        }
    }

    fn run(&mut self) -> anyhow::Result<()> {
        loop {
            while let Ok(host_event) = self.rx.try_recv() {
                match host_event {
                    HostEvent::Stop => self.stop_script().unwrap(),
                    HostEvent::LoadScript { name, chunk } => self.load_script(&name, &chunk)?,
                    HostEvent::Discover(device_names) => {
                        if self.ctx.has_script() {
                            self.ctx.call("on_discover", device_names)?
                        }
                    }
                    HostEvent::Connect(device_name) => {
                        if self.ctx.has_script() {
                            self.ctx.call("on_connect", device_name.as_str())?;
                        }
                        self.device_name = Some(device_name);
                    }
                    HostEvent::Midi(midi) => self.handle_midi(midi)?,
                    HostEvent::Terminate => {
                        self.stop_script()?;
                        return Ok(());
                    }
                }
            }
        }
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn load_script(&mut self, script_name: &str, chunk: &str) -> anyhow::Result<()> {
        self.stop_script()?;
        self.load_api(script_name)?;
        self.ctx.load_chunk(API)?;
        self.ctx.load_chunk(chunk)?;
        log::info!("script loaded : {script_name}");
        self.ctx.call("on_start", ())
    }

    fn handle_midi(&mut self, midi: MidiData) -> anyhow::Result<()> {
        if !self.ctx.has_script() || self.device_name.is_none() {
            return Ok(self.tx.try_send(ScriptEvent::Midi(midi))?);
        }

        let should_transfer: Option<bool> = self.ctx.call(
            "on_midi",
            (
                self.device_name.as_ref().unwrap().as_str(),
                midi.bytes.as_slice(),
            ),
        )?;

        if should_transfer.unwrap_or(true) {
            self.tx.try_send(ScriptEvent::Midi(midi))?;
        }

        Ok(())
    }

    fn load_api(&self, name: &str) -> anyhow::Result<()> {
        add_log(&self.ctx, name.to_owned(), self.tx.clone())?;
        add_connect(&self.ctx, name.to_owned(), self.tx.clone())?;
        add_alert(&self.ctx, name.to_owned(), self.tx.clone())?;
        add_resume(&self.ctx, name.to_owned(), self.tx.clone())?;
        add_pause(&self.ctx, name.to_owned(), self.tx.clone())?;
        add_stop(&self.ctx, name.to_owned(), self.tx.clone())
    }

    fn stop_script(&mut self) -> anyhow::Result<()> {
        if self.ctx.has_script() {
            self.ctx.call("on_stop", ())?;
        }

        self.ctx.release_script();
        Ok(())
    }
}

pub fn add_log(lua: &LuaEngine, name: String, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
    lua.set_fn("log", {
        move |_, message: String| {
            if let Err(e) = tx.try_send(ScriptEvent::Log(message)) {
                log::error!("{name} ! failed to send log event : {}", e);
            }
            Ok(())
        }
    })
}

pub fn add_connect(lua: &LuaEngine, name: String, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
    lua.set_fn("connect", {
        move |_, device_name: String| {
            if let Err(e) = tx.try_send(ScriptEvent::Connect(device_name)) {
                log::error!("{name} ! failed to send connection event : {}", e);
            }
            Ok(())
        }
    })
}

pub fn add_alert(lua: &LuaEngine, name: String, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
    lua.set_fn("alert", {
        move |_, device_name: String| {
            if let Err(e) = tx.try_send(ScriptEvent::Alert(device_name)) {
                log::error!("{name} ! failed to send alert event : {}", e);
            }
            Ok(())
        }
    })
}

pub fn add_pause(lua: &LuaEngine, name: String, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
    lua.set_fn("pause", {
        move |_, (): ()| {
            if let Err(e) = tx.try_send(ScriptEvent::Pause) {
                log::error!("{name} ! failed to send pause event : {}", e);
            }
            Ok(())
        }
    })
}

pub fn add_resume(lua: &LuaEngine, name: String, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
    lua.set_fn("resume", {
        move |_, (): ()| {
            if let Err(e) = tx.try_send(ScriptEvent::Resume) {
                log::error!("{name} ! failed to send resume event : {}", e);
            }
            Ok(())
        }
    })
}

pub fn add_stop(lua: &LuaEngine, name: String, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
    lua.set_fn("stop", {
        move |_, (): ()| {
            if let Err(e) = tx.try_send(ScriptEvent::Stop) {
                log::error!("{name} ! failed to send stop event : {}", e);
            }
            Ok(())
        }
    })
}
