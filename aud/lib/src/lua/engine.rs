use super::{
    handle::{start_engine, LuaEngineEvent, LuaEngineHandle, LuaRuntimeControlling},
    traits::{api::*, hooks::*},
    LuaRuntime,
};
use crate::{audio::AudioBuffer, files, midi::MidiData};
use crossbeam::channel::{Receiver, Sender};
use std::path::{Path, PathBuf};

pub enum HostEvent {
    LoadScript { name: String, chunk: String },
    Discover(Vec<String>),
    Connect(String),
    Midi(MidiData),
    Audio(AudioBuffer),
    Stop,
    Terminate,
}

pub enum ScriptEvent {
    Midi(MidiData),
    Log(LogApiEvent),
    Control(ControlFlowApiEvent),
    Connect(ConnectionApiEvent),
    Loaded,
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
pub struct ScriptLoader {
    tx: Sender<ScriptEvent>,
    rx: Receiver<HostEvent>,
    device_name: Option<String>,
    chunk_to_preload: &'static str,
}

impl ScriptLoader {
    pub fn new(
        tx: Sender<ScriptEvent>,
        rx: Receiver<HostEvent>,
        chunk_to_preload: &'static str,
    ) -> Self {
        Self {
            tx,
            rx,
            device_name: None,
            chunk_to_preload,
        }
    }

    fn load_script(&mut self, lua: &mut LuaRuntime, name: &str, chunk: &str) -> anyhow::Result<()> {
        self.stop_script(lua)?;
        lua.load_log(name.to_owned(), self.tx.clone())?;
        lua.load_alert(name.to_owned(), self.tx.clone())?;
        lua.load_connect(name.to_owned(), self.tx.clone())?;
        lua.load_resume(name.to_owned(), self.tx.clone())?;
        lua.load_pause(name.to_owned(), self.tx.clone())?;
        lua.load_stop(name.to_owned(), self.tx.clone())?;
        lua.load_chunk(self.chunk_to_preload)?;
        lua.load_chunk(chunk)?;
        log::trace!("script loaded : {name}");
        lua.on_start()
    }

    fn stop_script(&mut self, lua: &mut LuaRuntime) -> anyhow::Result<()> {
        lua.on_stop()?;
        let _ = lua.release_script();
        log::trace!("script released");
        Ok(())
    }

    fn handle_midi(&mut self, lua: &LuaRuntime, midi: MidiData) -> anyhow::Result<()> {
        let device_name = self.device_name.as_ref().map_or("", |s| s.as_str());

        if lua
            .on_midi(device_name, midi.bytes.as_slice())?
            .unwrap_or(true)
        {
            self.tx.try_send(ScriptEvent::Midi(midi))?;
        }

        Ok(())
    }

    fn handle_audio(&mut self, lua: &LuaRuntime, audio: AudioBuffer) -> anyhow::Result<()> {
        let device_name = self.device_name.as_ref().map_or("", |s| s.as_str());
        let audio = audio.deinterleave();
        lua.on_audio(device_name, &audio)?;
        Ok(())
    }
}

impl LuaRuntimeControlling for ScriptLoader {
    fn run(&mut self, lua: &mut LuaRuntime) -> anyhow::Result<()> {
        loop {
            while let Ok(event) = self.rx.recv() {
                match event {
                    HostEvent::Stop => self.stop_script(lua)?,
                    HostEvent::LoadScript { name, chunk } => {
                        self.load_script(lua, &name, &chunk)?;
                        self.tx.send(ScriptEvent::Loaded)?
                    }
                    HostEvent::Discover(device_names) => lua.on_discover(&device_names)?,
                    HostEvent::Connect(device_name) => {
                        lua.on_connect(device_name.as_str())?;
                        self.device_name = Some(device_name);
                    }
                    HostEvent::Midi(midi) => self.handle_midi(lua, midi)?,
                    HostEvent::Audio(audio) => self.handle_audio(lua, audio)?,
                    HostEvent::Terminate => {
                        self.stop_script(lua).unwrap();
                        return Ok(());
                    }
                }
            }
        }
    }
}

pub struct ScriptController {
    host_tx: Sender<HostEvent>,
    script_rx: Receiver<ScriptEvent>,
    lua_handle: LuaEngineHandle,
    script_path: Option<PathBuf>,
    file_watcher: Option<files::FsWatcher>,
}

impl ScriptController {
    pub fn start(chunk_to_preload: &'static str) -> Self {
        let (host_tx, host_rx) = crossbeam::channel::bounded::<HostEvent>(1_000);
        let (script_tx, script_rx) = crossbeam::channel::bounded::<ScriptEvent>(1_000);
        let loader = ScriptLoader::new(script_tx, host_rx, chunk_to_preload);

        Self {
            host_tx,
            script_rx,
            lua_handle: start_engine(loader),
            script_path: None,
            file_watcher: None,
        }
    }

    pub fn try_send(&self, host_event: HostEvent) -> anyhow::Result<()> {
        Ok(self.host_tx.try_send(host_event)?)
    }

    pub fn try_recv(&self) -> anyhow::Result<ScriptEvent> {
        Ok(self.script_rx.try_recv()?)
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.script_path.as_ref()
    }

    pub fn name(&self) -> Option<&str> {
        self.path()?.file_name()?.to_str()
    }

    pub fn try_recv_engine_events(&self) -> anyhow::Result<LuaEngineEvent> {
        Ok(self.lua_handle.events().try_recv()?)
    }

    pub fn load(&mut self, script: impl AsRef<Path>) -> anyhow::Result<()> {
        let script_path = script.as_ref();
        if !script_path.exists() || !script_path.is_file() {
            anyhow::bail!("Invalid script path or type");
        }

        self.script_path = Some(script_path.into());

        if let Err(e) = self.host_tx.try_send(HostEvent::Stop) {
            log::error!("failed to send stop event : {e}");
        }

        let event = HostEvent::LoadScript {
            name: self
                .script_path
                .as_ref()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
            chunk: std::fs::read_to_string(script_path)?,
        };

        self.file_watcher = files::FsWatcher::run(script_path).ok();

        if let Err(e) = self.host_tx.try_send(event) {
            log::error!("failed to send load script event : {e}");
        }

        Ok(())
    }

    pub fn was_script_modified(&mut self) -> anyhow::Result<bool> {
        let Some(ref watcher) = self.file_watcher else {
            return Ok(false);
        };

        for event in watcher.events().try_iter().collect::<Vec<_>>() {
            if self.has_file_changed(event) {
                if self.script_path.is_some() {
                    log::trace!("Loaded script has changed on filesystem");
                    return Ok(true);
                }

                break;
            }
        }

        Ok(false)
    }

    fn has_file_changed(&mut self, event: notify::Result<notify::Event>) -> bool {
        match event {
            Ok(event) => matches!(event.kind, notify::EventKind::Modify(_)),
            Err(e) => {
                log::error!("Script reload failed : {e}");
                false
            }
        }
    }
}

impl Drop for ScriptController {
    fn drop(&mut self) {
        let Some(handle) = self.lua_handle.take_handle() else {
            return;
        };

        if let Err(e) = self.host_tx.try_send(HostEvent::Terminate) {
            log::error!("Failed to send termination message to Lua runtime : {e}");
            return;
        };

        if handle.join().is_err() {
            log::error!("Failed to join on Lua runtime thread handle");
        }
    }
}
