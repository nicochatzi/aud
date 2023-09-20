use crate::{
    lua::{
        traits::{api::*, hooks::*},
        LuaRuntime, LuaRuntimeControlling,
    },
    streams::midi::MidiData,
};
use crossbeam::channel::{Receiver, Sender};

pub const API: &str = include_str!("../../../../../api/midimon/api.lua");
pub const DOCS: &str = include_str!("../../../../../api/midimon/docs.lua");

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

#[derive(Clone)]
pub struct ScriptController {
    tx: Sender<ScriptEvent>,
    rx: Receiver<HostEvent>,
    device_name: Option<String>,
}

impl ScriptController {
    pub fn new(tx: Sender<ScriptEvent>, rx: Receiver<HostEvent>) -> Self {
        Self {
            tx,
            rx,
            device_name: None,
        }
    }
}

impl LuaRuntimeControlling for ScriptController {
    fn run(&mut self, lua: &mut LuaRuntime) -> anyhow::Result<()> {
        loop {
            while let Ok(event) = self.rx.recv() {
                match event {
                    HostEvent::Stop => self.stop_script(lua)?,
                    HostEvent::LoadScript { name, chunk } => {
                        self.load_script(lua, &name, &chunk)?
                    }
                    HostEvent::Discover(device_names) => lua.on_discover(&device_names)?,
                    HostEvent::Connect(device_name) => {
                        lua.on_connect(device_name.as_str())?;
                        self.device_name = Some(device_name);
                    }
                    HostEvent::Midi(midi) => self.handle_midi(lua, midi)?,
                    HostEvent::Terminate => {
                        self.stop_script(lua).unwrap();
                        return Ok(());
                    }
                }
            }
        }
    }
}

impl ScriptController {
    fn load_script(&mut self, lua: &mut LuaRuntime, name: &str, chunk: &str) -> anyhow::Result<()> {
        self.stop_script(lua)?;
        lua.load_log(name.to_owned(), self.tx.clone())?;
        lua.load_alert(name.to_owned(), self.tx.clone())?;
        lua.load_connect(name.to_owned(), self.tx.clone())?;
        lua.load_resume(name.to_owned(), self.tx.clone())?;
        lua.load_pause(name.to_owned(), self.tx.clone())?;
        lua.load_stop(name.to_owned(), self.tx.clone())?;
        lua.load_chunk(API)?;
        lua.load_chunk(chunk)?;
        log::info!("script loaded : {name}");
        lua.on_start()
    }

    fn stop_script(&mut self, lua: &mut LuaRuntime) -> anyhow::Result<()> {
        lua.on_stop()?;
        let _ = lua.release_script();
        Ok(())
    }

    fn handle_midi(&mut self, lua: &LuaRuntime, midi: MidiData) -> anyhow::Result<()> {
        let device_name = if self.device_name.is_some() {
            self.device_name.as_ref().unwrap().as_str()
        } else {
            ""
        };

        if lua
            .on_midi(device_name, midi.bytes.as_slice())?
            .unwrap_or(true)
        {
            self.tx.try_send(ScriptEvent::Midi(midi))?;
        }

        Ok(())
    }
}
