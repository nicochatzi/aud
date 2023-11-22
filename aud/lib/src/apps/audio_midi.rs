use crate::{
    audio::{
        AudioBuffer, AudioChannelSelection, AudioDevice, AudioInterface, AudioProviding,
        HostAudioInput,
    },
    lua::{imported, traits::api::*, HostEvent, LuaEngineEvent, ScriptController, ScriptEvent},
    midi::{HostedMidiReceiver, MidiData, MidiReceiving},
};
use std::path::{Path, PathBuf};

pub trait AudioProvider: AudioProviding + AudioInterface {}

impl<T> AudioProvider for T where T: AudioProviding + AudioInterface {}

#[derive(Debug, PartialEq, Eq)]
pub enum AppEvent {
    Continue,
    Stopping,
    ScriptCrash,
    ScriptLoaded,
}

pub struct AudioApp {
    receiver: Box<dyn AudioProvider>,
    buffer: AudioBuffer,
    selected_device: Option<AudioDevice>,
    selected_channels: Option<AudioChannelSelection>,
}

impl AudioApp {
    pub fn new(receiver: Box<dyn AudioProvider>) -> Self {
        Self {
            buffer: AudioBuffer::default(),
            receiver,
            selected_device: None,
            selected_channels: None,
        }
    }

    pub fn devices(&self) -> &[AudioDevice] {
        self.receiver.list_audio_devices()
    }

    pub fn buffer(&self) -> &AudioBuffer {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut AudioBuffer {
        &mut self.buffer
    }

    pub fn selected_device(&self) -> Option<&AudioDevice> {
        self.selected_device.as_ref()
    }

    pub fn selected_channels(&self) -> Option<&AudioChannelSelection> {
        self.selected_channels.as_ref()
    }

    pub fn fetch_buffer(&mut self) -> anyhow::Result<()> {
        self.receiver.process_audio_events()?;
        let mut audio = self.receiver.retrieve_audio_buffer();
        if self.buffer.num_channels != audio.num_channels {
            self.buffer = audio;
        } else {
            self.buffer.data.append(&mut audio.data);
        }
        Ok(())
    }

    fn update_channel_selection(
        &mut self,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        let Some(ref audio_device) = self.selected_device else {
            anyhow::bail!("No audio device selected");
        };
        self.buffer.data.clear();
        self.receiver
            .connect_to_audio_device(audio_device, channel_selection.clone())?;
        self.selected_channels = Some(channel_selection);
        Ok(())
    }

    fn connect_to_input(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        self.selected_device = Some(audio_device.clone());
        self.buffer.num_channels = channel_selection.count() as u32;
        self.update_channel_selection(channel_selection)
    }
}

pub struct MidiApp {
    receiver: Box<dyn MidiReceiving>,
    port_names: Vec<String>,
    selected_port_name: Option<String>,
    messages: Vec<MidiData>,
}

impl MidiApp {
    pub fn new(receiver: Box<dyn MidiReceiving>) -> Self {
        Self {
            port_names: receiver.list_midi_devices().unwrap(),
            receiver,
            selected_port_name: None,
            messages: vec![],
        }
    }

    pub fn is_runing(&self) -> bool {
        self.receiver.is_midi_stream_active()
    }

    pub fn set_running(&mut self, should_run: bool) {
        self.receiver.set_midi_stream_active(should_run)
    }

    pub fn midi_ports(&self) -> &[String] {
        self.port_names.as_slice()
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    pub fn selected_port(&self) -> Option<&str> {
        self.selected_port_name.as_deref()
    }

    pub fn take_messages(&mut self) -> Vec<MidiData> {
        std::mem::take(&mut self.messages)
    }
}

pub struct App {
    audio: AudioApp,
    midi: MidiApp,
    script: ScriptController,
    alert_message: Option<String>,
}

impl App {
    pub fn new(
        audio_receiver: Box<dyn AudioProvider>,
        midi_receiver: Box<dyn MidiReceiving>,
    ) -> Self {
        Self {
            audio: AudioApp::new(audio_receiver),
            midi: MidiApp::new(midi_receiver),
            script: ScriptController::start(imported::midimon::API),
            alert_message: None,
        }
    }

    pub fn with_audio(audio_receiver: Box<dyn AudioProvider>) -> Self {
        Self::new(audio_receiver, Box::<HostedMidiReceiver>::default())
    }

    pub fn with_midi(midi_receiver: Box<dyn MidiReceiving>) -> Self {
        Self::new(Box::<HostAudioInput>::default(), midi_receiver)
    }

    pub fn audio(&self) -> &AudioApp {
        &self.audio
    }

    pub fn audio_mut(&mut self) -> &mut AudioApp {
        &mut self.audio
    }

    pub fn midi(&self) -> &MidiApp {
        &self.midi
    }

    pub fn midi_mut(&mut self) -> &mut MidiApp {
        &mut self.midi
    }

    pub fn take_alert(&mut self) -> Option<String> {
        self.alert_message.take()
    }

    pub fn selected_script(&self) -> Option<&str> {
        self.script.name().as_deref()
    }

    pub fn loaded_script_path(&self) -> Option<&PathBuf> {
        self.script.path()
    }

    pub fn connect_to_midi_input_by_index(&mut self, port_index: usize) -> anyhow::Result<()> {
        if self.midi.port_names.get(port_index).is_none() {
            return Ok(());
        }
        self.connect_to_midi_input_by_index_unchecked(port_index)?;
        self.midi.clear_messages();
        Ok(())
    }

    pub fn connect_to_midi_input(&mut self, port_name: &str) -> anyhow::Result<()> {
        match self
            .midi_port_names
            .iter()
            .position(|name| name == port_name)
        {
            Some(index) => self.connect_to_midi_input_by_index(index),
            None => Ok(()),
        }
    }

    /// Send a script to be loaded by the scripting engine. This function does not block.
    pub fn load_script(&mut self, script: impl AsRef<Path>) -> anyhow::Result<AppEvent> {
        self.script.load(script)?;

        if self.midi.selected_port().is_some() {
            self.send_midi_port_discovery()?;
        }

        if self.audio.selected_device().is_some() {
            self.send_audio_device_discovery()?;
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
        for msg in self.midi_receiver.produce_midi_messages() {
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
                ScriptEvent::Midi(midi) => self.midi.messages_mut().push(midi),
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
        let ConnectionApiEvent { ref device } = request;
        if self.midi_port_names.iter().any(|name| name == device) {
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

    fn send_midi_port_discovery(&mut self) -> anyhow::Result<()> {
        if let Err(e) = self
            .script
            .try_send(HostEvent::Discover(self.midi.port_names().clone()))
        {
            log::error!("failed to send discovery event : {e}");
        }

        if self.midi.selected_port().is_some() {
            let port = self.midi.selected_port().as_ref().unwrap().clone();
            self.connect_to_midi_input(&port)?;
        }

        Ok(())
    }

    fn send_audio_device_discovery(&mut self) -> anyhow::Result<()> {
        if let Err(e) = self.script.try_send(HostEvent::Discover(
            self.audio_devices()
                .iter()
                .map(|dev| dev.name.clone())
                .collect(),
        )) {
            log::error!("failed to send discovery event : {e}");
        }

        if self.selected_audio_device.is_some() && self.selected_audio_channels.is_some() {
            self.connect_to_audio_input(
                &self.selected_audio_device.as_ref().unwrap().clone(),
                self.selected_audio_channels.as_ref().unwrap().clone(),
            )?;
        }

        Ok(())
    }

    fn connect_to_midi_input_by_index_unchecked(&mut self, index: usize) -> anyhow::Result<()> {
        let port_name = &self.midi_port_names[index];
        self.midi_receiver.connect_to_midi_device(port_name)?;
        self.selected_midi_port_name = Some(port_name.into());
        let port_name = port_name.to_owned();

        if let Err(e) = self.script.try_send(HostEvent::Connect(port_name)) {
            log::error!("Failed to send device connected event to runtime : {e}");
        }

        Ok(())
    }
}
