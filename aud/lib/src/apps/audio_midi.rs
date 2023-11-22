use super::{
    audio::{AudioProvider, AudioProviderController},
    midi::MidiReceiverController,
};
use crate::{
    audio::{AudioChannelSelection, HostAudioInput},
    lua::{traits::api::*, HostEvent, LuaEngineEvent, ScriptController, ScriptEvent},
    midi::{HostedMidiReceiver, MidiReceiving},
};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Debug, PartialEq, Eq)]
pub enum AppEvent {
    Continue,
    Stopping,
    ScriptCrash,
    ScriptLoaded,
}

pub struct AudioMidiController {
    audio: AudioProviderController,
    midi: MidiReceiverController,
    script: Rc<RefCell<ScriptController>>,
    alert_message: Option<String>,
}

impl AudioMidiController {
    pub fn new(
        audio_receiver: Box<dyn AudioProvider>,
        midi_receiver: Box<dyn MidiReceiving>,
        script_api: &'static str,
    ) -> Self {
        let script = Rc::new(RefCell::new(ScriptController::start(script_api)));

        Self {
            audio: AudioProviderController::new(audio_receiver, script.clone()),
            midi: MidiReceiverController::new(midi_receiver, script.clone()),
            script,
            alert_message: None,
        }
    }

    pub fn with_audio(audio_receiver: Box<dyn AudioProvider>, script_api: &'static str) -> Self {
        Self::new(
            audio_receiver,
            Box::<HostedMidiReceiver>::default(),
            script_api,
        )
    }

    pub fn with_midi(midi_receiver: Box<dyn MidiReceiving>, script_api: &'static str) -> Self {
        Self::new(Box::<HostAudioInput>::default(), midi_receiver, script_api)
    }

    pub fn audio(&self) -> &AudioProviderController {
        &self.audio
    }

    pub fn audio_mut(&mut self) -> &mut AudioProviderController {
        &mut self.audio
    }

    pub fn midi(&self) -> &MidiReceiverController {
        &self.midi
    }

    pub fn midi_mut(&mut self) -> &mut MidiReceiverController {
        &mut self.midi
    }

    pub fn take_alert(&mut self) -> Option<String> {
        self.alert_message.take()
    }

    pub fn selected_script(&self) -> Option<String> {
        self.script.borrow().name().map(str::to_owned)
    }

    pub fn loaded_script_path(&self) -> Option<PathBuf> {
        self.script.borrow().path().map(PathBuf::from)
    }

    /// Send a script to be loaded by the scripting engine. This function does not block.
    pub fn load_script(&mut self, script_path: impl AsRef<Path>) -> anyhow::Result<AppEvent> {
        self.script.borrow_mut().load(script_path)?;

        if self.midi.selected_port().is_some() {
            self.send_midi_port_discovery()?;
            self.midi.reconnect()?;
        }

        if self.audio.selected_device().is_some() {
            self.send_audio_device_discovery();
            self.audio.reconnect()?;
        }

        Ok(AppEvent::Continue)
    }

    /// Load a script and block until the script has been loaded by the engine.
    pub fn load_script_sync(
        &mut self,
        script_path: impl AsRef<Path>,
        timeout: std::time::Duration,
    ) -> anyhow::Result<()> {
        self.load_script(script_path)?;
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

    /// Process all the available script events without blocking.
    /// This processes all the available events unless the engine:
    /// - requests to stop the application
    /// - has just loaded a script
    pub fn process_script_events(&mut self) -> anyhow::Result<AppEvent> {
        loop {
            let event = {
                match self.script.borrow().try_recv() {
                    Ok(event) => event,
                    Err(_) => break,
                }
            };

            self.process_script_event(event)?;
        }

        Ok(AppEvent::Continue)
    }

    fn process_script_event(&mut self, event: ScriptEvent) -> anyhow::Result<AppEvent> {
        match event {
            ScriptEvent::Loaded => return Ok(AppEvent::ScriptLoaded),
            ScriptEvent::Log(request) => self.handle_lua_log_request(request),
            ScriptEvent::Midi(midi) => self.midi.push_message(midi),
            ScriptEvent::Connect(request) => self.handle_lua_connect_request(request)?,
            ScriptEvent::Control(request) => {
                if self.handle_lua_control_request(request) == AppEvent::Stopping {
                    return Ok(AppEvent::Stopping);
                }
            }
        }

        Ok(AppEvent::Continue)
    }

    /// Process all the available file watcher events without blocking.
    pub fn process_file_events(&mut self) -> anyhow::Result<AppEvent> {
        let (was_modified, script_path) = {
            let script = self.script.borrow();
            (script.was_script_modified()?, script.path().cloned())
        };

        if was_modified {
            if let Some(path) = script_path {
                self.load_script(path)
            } else {
                Ok(AppEvent::Continue)
            }
        } else {
            Ok(AppEvent::Continue)
        }
    }

    /// Process all the available engine events without blocking.
    pub fn process_engine_events(&mut self) -> anyhow::Result<AppEvent> {
        while let Ok(event) = self.script.borrow().try_recv_engine_events() {
            match event {
                LuaEngineEvent::Panicked => return Ok(AppEvent::ScriptCrash),
                LuaEngineEvent::Terminated => log::info!("Lua Engine terminated"),
            }
        }
        Ok(AppEvent::Continue)
    }

    fn handle_lua_connect_request(&mut self, request: ConnectionApiEvent) -> anyhow::Result<()> {
        let ConnectionApiEvent {
            ref device,
            channels: _,
        } = request;

        if self.midi.port_names().iter().any(|name| name == device) {
            self.midi.connect_to_input(device)?;
            return Ok(());
        }

        let device = self
            .audio
            .devices()
            .iter()
            .find(|dev| dev.name == *device)
            .cloned();

        if let Some(device) = device {
            let channels = self
                .audio
                .selected_channels()
                .unwrap_or(&AudioChannelSelection::Mono(0));

            self.audio.connect_to_input(&device, channels.clone())?;
        }

        Ok(())
    }

    fn handle_lua_control_request(&mut self, request: ControlFlowApiEvent) -> AppEvent {
        match request {
            ControlFlowApiEvent::Pause => self.midi.set_running(false),
            ControlFlowApiEvent::Resume => self.midi.set_running(true),
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

    fn send_midi_port_discovery(&mut self) -> anyhow::Result<()> {
        if let Err(e) = self
            .script
            .borrow()
            .try_send(HostEvent::Discover(self.midi.port_names().to_vec()))
        {
            log::error!("failed to send discovery event : {e}");
        }

        Ok(())
    }

    fn send_audio_device_discovery(&mut self) {
        let devices = self
            .audio
            .devices()
            .iter()
            .map(|dev| dev.name.clone())
            .collect();

        if let Err(e) = self.script.borrow().try_send(HostEvent::Discover(devices)) {
            log::error!("failed to send discovery event : {e}");
        }
    }
}
