use crate::{
    lua::{imported, traits::api::*, HostEvent, LuaEngineEvent, ScriptController, ScriptEvent},
    midi::{MidiData, MidiReceiving},
};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum AppEvent {
    Continue,
    Stopping,
    ScriptCrash,
    ScriptLoaded,
}

pub struct App {
    midi_in: Box<dyn MidiReceiving>,
    script: ScriptController,
    port_names: Vec<String>,
    selected_port_name: Option<String>,
    selected_script_name: Option<String>,
    alert_message: Option<String>,
    messages: Vec<MidiData>,
}

impl App {
    pub fn new(midi_in: Box<dyn MidiReceiving>) -> Self {
        Self {
            port_names: midi_in.list_midi_devices().unwrap(),
            midi_in,
            script: ScriptController::start(imported::midimon::API),
            selected_port_name: None,
            selected_script_name: None,
            alert_message: None,
            messages: vec![],
        }
    }

    pub fn running(&self) -> bool {
        self.midi_in.is_midi_stream_active()
    }

    pub fn set_running(&mut self, should_run: bool) {
        self.midi_in.set_midi_stream_active(should_run)
    }

    pub fn ports(&self) -> &[String] {
        self.port_names.as_slice()
    }

    pub fn take_alert(&mut self) -> Option<String> {
        self.alert_message.take()
    }

    pub fn selected_port(&self) -> Option<&str> {
        self.selected_port_name.as_deref()
    }

    pub fn selected_script(&self) -> Option<&str> {
        self.selected_script_name.as_deref()
    }

    pub fn loaded_script_path(&self) -> Option<&PathBuf> {
        self.script.path()
    }

    pub fn take_messages(&mut self) -> Vec<MidiData> {
        std::mem::take(&mut self.messages)
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    pub fn connect_to_midi_input_by_index(&mut self, port_index: usize) -> anyhow::Result<()> {
        if self.port_names.get(port_index).is_none() {
            return Ok(());
        }
        {
            let port_name = &self.port_names[port_index];
            self.midi_in.connect_to_midi_device(port_name)?;
            self.selected_port_name = Some(port_name.into());
            let port_name = port_name.to_owned();

            if let Err(e) = self.script.try_send(HostEvent::Connect(port_name)) {
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

    /// Send a script to be loaded by the scripting engine. This function does not block.
    pub fn load_script(&mut self, script: impl AsRef<Path>) -> anyhow::Result<AppEvent> {
        let script = script.as_ref();
        self.script.load(script)?;

        self.selected_script_name = Some(
            script
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
                .to_owned(),
        );

        if let Err(e) = self
            .script
            .try_send(HostEvent::Discover(self.port_names.clone()))
        {
            log::error!("failed to send discovery event : {e}");
        }

        if self.selected_port_name.is_some() {
            let port = self.selected_port_name.as_ref().unwrap().clone();
            self.connect_to_midi_input(&port)?;
        }

        Ok(AppEvent::Continue)
    }

    /// Load a script and block until the script has been loaded by the engine.
    pub fn load_script_sync(
        &mut self,
        script: impl AsRef<Path>,
        timeout: std::time::Duration,
    ) -> anyhow::Result<()> {
        self.load_script(script)?;
        let start = std::time::Instant::now();
        while self.process_script_events()? != AppEvent::ScriptLoaded {
            if start.elapsed() > timeout {
                anyhow::bail!("Failed to load script in time");
            }
        }
        Ok(())
    }

    /// Block while waiting for the script to push an alert back to the app.
    pub fn wait_for_alert(
        &mut self,
        timeout: std::time::Duration,
    ) -> anyhow::Result<Option<String>> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            let _ = self.process_script_events()?;
            if self.alert_message.is_some() {
                return Ok(self.take_alert());
            }
        }
        Ok(self.take_alert())
    }

    /// Transfer all received MIDI messages to the engine.
    pub fn process_midi_messages(&mut self) {
        for msg in self.midi_in.produce_midi_messages() {
            if let Err(e) = self.script.try_send(HostEvent::Midi(msg)) {
                log::error!("Failed to send midi to Lua Runtime : {e}");
            }
        }
    }

    /// Process all the available script events without blocking.
    /// This processes all the available events unless the engine:
    /// - requests to stop the application
    /// - has just loaded a script
    pub fn process_script_events(&mut self) -> anyhow::Result<AppEvent> {
        while let Ok(script_event) = self.script.try_recv() {
            match script_event {
                ScriptEvent::Loaded => return Ok(AppEvent::ScriptLoaded),
                ScriptEvent::Log(request) => self.handle_lua_log_request(request),
                ScriptEvent::Midi(midi) => self.messages.push(midi),
                ScriptEvent::Connect(request) => self.handle_lua_connect_request(request)?,
                ScriptEvent::Control(request) => {
                    if self.handle_lua_control_request(request) == AppEvent::Stopping {
                        return Ok(AppEvent::Stopping);
                    }
                }
            }
        }

        Ok(AppEvent::Continue)
    }

    /// Process all the available file watcher events without blocking.
    pub fn process_file_events(&mut self) -> anyhow::Result<AppEvent> {
        if self.script.was_script_modified()? && self.script.path().is_some() {
            self.load_script(self.script.path().unwrap().clone())
        } else {
            Ok(AppEvent::Continue)
        }
    }

    /// Process all the available engine events without blocking.
    pub fn process_engine_events(&mut self) -> anyhow::Result<AppEvent> {
        while let Ok(event) = self.script.try_recv_engine_events() {
            match event {
                LuaEngineEvent::Panicked => return Ok(AppEvent::ScriptCrash),
                LuaEngineEvent::Terminated => log::info!("Lua Engine terminated"),
            }
        }
        Ok(AppEvent::Continue)
    }

    fn handle_lua_connect_request(&mut self, request: ConnectionApiEvent) -> anyhow::Result<()> {
        let ConnectionApiEvent { ref device, .. } = request;
        if self.port_names.iter().any(|name| name == device) {
            self.connect_to_midi_input(device)?;
        }
        Ok(())
    }

    fn handle_lua_control_request(&mut self, request: ControlFlowApiEvent) -> AppEvent {
        match request {
            ControlFlowApiEvent::Pause => self.set_running(false),
            ControlFlowApiEvent::Resume => self.set_running(true),
            ControlFlowApiEvent::Stop => return AppEvent::Stopping,
        }
        AppEvent::Continue
    }

    fn handle_lua_log_request(&mut self, request: LogApiEvent) {
        match request {
            LogApiEvent::Log(msg) => log::info!("{msg}"),
            LogApiEvent::Alert(msg) => self.alert_message = Some(msg),
        }
    }
}
