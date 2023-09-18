use crate::lua::{
    traits::{api::*, hooks::*},
    LuaEngine,
};
use crossbeam::channel::{Receiver, Sender};

pub const API: &str = include_str!("../../../api/midimon/api.lua");
pub const DOCS: &str = include_str!("../../../api/midimon/docs.lua");

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
    Midi(MidiData),
    Log(LogApiEvent),
    Control(ControlFlowApiEvent),
    Connect(ConnectionApiEvent),
}

impl From<LogApiEvent> for ScriptEvent {
    fn from(event: LogApiEvent) -> Self {
        Self::Log(event)
    }
}

impl From<ControlFlowApiEvent> for ScriptEvent {
    fn from(event: ControlFlowApiEvent) -> Self {
        Self::Control(event)
    }
}

impl From<ConnectionApiEvent> for ScriptEvent {
    fn from(event: ConnectionApiEvent) -> Self {
        Self::Connect(event)
    }
}

pub struct LuaRuntime {
    ctx: LuaEngine,
    tx: Sender<ScriptEvent>,
    rx: Receiver<HostEvent>,
    device_name: Option<String>,
}

pub struct LuaRuntimeHandle {
    handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>,
    _tx: Sender<ScriptEvent>,
    _rx: Receiver<HostEvent>,
}

impl LuaRuntimeHandle {
    pub fn take_handle(&mut self) -> Option<std::thread::JoinHandle<anyhow::Result<()>>> {
        self.handle.take()
    }
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
            while let Ok(host_event) = self.rx.recv() {
                match host_event {
                    HostEvent::Stop => self.stop_script().unwrap(),
                    HostEvent::LoadScript { name, chunk } => self.load_script(&name, &chunk)?,
                    HostEvent::Discover(device_names) => self.ctx.on_discover(&device_names)?,
                    HostEvent::Connect(device_name) => {
                        self.ctx.on_connect(device_name.as_str())?;
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

    fn load_script(&mut self, script_name: &str, chunk: &str) -> anyhow::Result<()> {
        self.stop_script()?;
        self.load_api(script_name)?;
        self.ctx.load_chunk(API)?;
        self.ctx.load_chunk(chunk)?;
        log::info!("script loaded : {script_name}");
        self.ctx.on_start()
    }

    fn handle_midi(&mut self, midi: MidiData) -> anyhow::Result<()> {
        let device_name = if self.device_name.is_some() {
            self.device_name.as_ref().unwrap().as_str()
        } else {
            ""
        };

        if self
            .ctx
            .on_midi(device_name, midi.bytes.as_slice())?
            .unwrap_or(true)
        {
            self.tx.try_send(ScriptEvent::Midi(midi))?;
        }

        Ok(())
    }

    fn load_api(&self, name: &str) -> anyhow::Result<()> {
        self.ctx.load_log(name.to_owned(), self.tx.clone())?;
        self.ctx.load_alert(name.to_owned(), self.tx.clone())?;
        self.ctx.load_connect(name.to_owned(), self.tx.clone())?;
        self.ctx.load_resume(name.to_owned(), self.tx.clone())?;
        self.ctx.load_pause(name.to_owned(), self.tx.clone())?;
        self.ctx.load_stop(name.to_owned(), self.tx.clone())
    }

    fn stop_script(&mut self) -> anyhow::Result<()> {
        self.ctx.on_stop()?;
        let _ = self.ctx.release_script();
        Ok(())
    }
}
