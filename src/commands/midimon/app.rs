use super::lua::*;
use crate::{
    lua::{traits::api::*, LuaEngineHandle},
    midi::MidiMessageString,
};
use crossbeam::channel::{Receiver, Sender};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub struct App {
    is_running: Arc<AtomicBool>,

    host_tx: Sender<HostEvent>,
    script_rx: Receiver<ScriptEvent>,
    lua_handle: LuaEngineHandle,
    midi_in: crate::midi::Input<Sender<HostEvent>>,

    port_names: Vec<String>,
    selected_port_name: Option<String>,
    selected_script_name: Option<String>,

    alert_message: Option<String>,
    messages: Vec<MidiMessageString>,

    script_path: Option<PathBuf>,
    file_events: Option<Receiver<notify::Result<notify::Event>>>,
}

impl Default for App {
    fn default() -> Self {
        let (host_tx, host_rx) = crossbeam::channel::bounded::<HostEvent>(1_000);
        let (script_tx, script_rx) = crossbeam::channel::bounded::<ScriptEvent>(1_000);
        let midi_in = crate::midi::Input::default();

        Self {
            is_running: Arc::new(AtomicBool::new(true)),
            host_tx,
            script_rx,
            lua_handle: crate::lua::start_engine(ScriptController::new(script_tx, host_rx)),
            selected_port_name: None,
            port_names: midi_in.port_names().unwrap(),
            midi_in,
            selected_script_name: None,
            alert_message: None,
            messages: vec![],
            script_path: None,
            file_events: None,
        }
    }
}

impl App {
    pub fn running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn set_running(&mut self, should_run: bool) {
        self.is_running.store(should_run, Ordering::SeqCst)
    }

    pub fn ports(&self) -> &[String] {
        self.port_names.as_slice()
    }

    pub fn take_alert(&mut self) -> Option<String> {
        self.alert_message.take()
    }

    pub fn selected_port(&self) -> Option<&String> {
        self.selected_port_name.as_ref()
    }

    pub fn selected_script(&self) -> Option<&String> {
        self.selected_script_name.as_ref()
    }

    pub fn loaded_script_path(&self) -> Option<&PathBuf> {
        self.script_path.as_ref()
    }

    pub fn messages(&self) -> &[MidiMessageString] {
        self.messages.as_slice()
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    pub fn update_ports(&mut self) -> anyhow::Result<()> {
        self.port_names = self.midi_in.port_names()?;
        Ok(())
    }

    pub fn connect_to_midi_input_by_index(&mut self, port_index: usize) -> anyhow::Result<()> {
        if self.port_names.get(port_index).is_none() {
            return Ok(());
        }

        {
            let port_name = &self.port_names[port_index];
            self.midi_in.select(port_name)?;
            self.midi_in.connect(
                {
                    let is_running = self.is_running.clone();
                    move |timestamp, bytes, sender| {
                        if !is_running.load(Ordering::SeqCst) {
                            return;
                        }

                        let midi = MidiData {
                            timestamp,
                            bytes: bytes.into(),
                        };

                        if let Err(e) = sender.try_send(HostEvent::Midi(midi)) {
                            log::error!("Failed to push midi message event to runtime : {e}");
                        }
                    }
                },
                self.host_tx.clone(),
            )?;
            self.selected_port_name = Some(port_name.into());
            let port_name = port_name.to_owned();
            if let Err(e) = self.host_tx.try_send(HostEvent::Connect(port_name)) {
                log::error!("Failed to send device connected event to runtime : {e}");
            }
        }

        self.clear_messages();

        Ok(())
    }

    pub fn connect_to_midi_input(&mut self, port_name: &str) -> anyhow::Result<()> {
        match self.port_names.iter().position(|name| name == port_name) {
            Some(index) => self.connect_to_midi_input_by_index(index),
            None => Ok(()),
        }
    }

    pub fn load_script(&mut self, script: impl AsRef<Path>) -> anyhow::Result<bool> {
        let script_path = script.as_ref();
        if !script_path.exists() || !script_path.is_file() {
            anyhow::bail!("Invalid script path or type");
        }

        self.script_path = Some(script_path.into());
        self.selected_script_name = Some(
            script_path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
                .to_owned(),
        );

        if let Err(e) = self.host_tx.try_send(HostEvent::Stop) {
            log::error!("failed to send stop event : {e}");
        }

        let event = HostEvent::LoadScript {
            name: self.selected_script_name.as_ref().unwrap().to_owned(),
            chunk: std::fs::read_to_string(script_path)?,
        };

        self.file_events = crate::file::watch(script_path).ok();

        if let Err(e) = self.host_tx.try_send(event) {
            log::error!("failed to send load script event : {e}");
        }

        if let Err(e) = self
            .host_tx
            .try_send(HostEvent::Discover(self.port_names.clone()))
        {
            log::error!("failed to send discovery event : {e}");
        }

        if self.midi_in.is_connected() && self.selected_port_name.is_some() {
            let port = self.selected_port_name.as_ref().unwrap().clone();
            self.connect_to_midi_input(&port)?;
        }

        Ok(true)
    }

    pub fn process_script_events(&mut self) -> anyhow::Result<bool> {
        while let Ok(script_event) = self.script_rx.try_recv() {
            match script_event {
                ScriptEvent::Connect(request) => self.handle_lua_connect_request(request)?,
                ScriptEvent::Control(request) => {
                    if !self.handle_lua_control_request(request) {
                        return Ok(false);
                    }
                }
                ScriptEvent::Log(request) => self.handle_lua_log_request(request),
                ScriptEvent::Midi(midi) => {
                    if let Some(midi) = MidiMessageString::new(midi.timestamp, &midi.bytes) {
                        self.messages.push(midi)
                    }
                }
            }
        }

        Ok(true)
    }

    pub fn process_file_events(&mut self) -> anyhow::Result<()> {
        let Some(ref file_events) = self.file_events else {
            return Ok(());
        };

        // consume all the events without blocking
        let events: Vec<_> = file_events.try_iter().collect();
        for event in events {
            if self.has_file_changed(event) {
                if let Some(script) = self.script_path.clone() {
                    self.load_script(script)?;
                }

                break;
            }
        }

        Ok(())
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

    fn handle_lua_connect_request(&mut self, request: ConnectionApiEvent) -> anyhow::Result<()> {
        let ConnectionApiEvent { ref device } = request;

        if self.port_names.iter().any(|name| name == device) {
            self.connect_to_midi_input(device)?;
        }

        Ok(())
    }

    fn handle_lua_control_request(&mut self, request: ControlFlowApiEvent) -> bool {
        match request {
            ControlFlowApiEvent::Pause => self.set_running(false),
            ControlFlowApiEvent::Resume => self.set_running(true),
            ControlFlowApiEvent::Stop => return false,
        }

        true
    }

    fn handle_lua_log_request(&mut self, request: LogApiEvent) {
        match request {
            LogApiEvent::Log(msg) => log::info!("{msg}"),
            LogApiEvent::Alert(msg) => self.alert_message = Some(msg),
        }
    }
}

impl Drop for App {
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
