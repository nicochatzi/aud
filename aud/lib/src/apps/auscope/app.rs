use crate::{
    audio::*,
    comms::{SocketInterface, Sockets},
    lua::{
        imported,
        traits::api::{ConnectionApiEvent, LogApiEvent},
        HostEvent, LuaEngineEvent, ScriptController, ScriptEvent,
    },
};
use crossbeam::channel;
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

pub struct App {
    buffer: AudioBuffer,
    receiver: Box<dyn AudioProvider>,
    selected_device: Option<AudioDevice>,
    selected_channels: Option<AudioChannelSelection>,
    selected_script_name: Option<String>,
    script: ScriptController,
    alert_message: Option<String>,
}

impl App {
    pub fn new(audio_receiver: Box<dyn AudioProvider>) -> Self {
        Self {
            buffer: AudioBuffer::default(),
            script: ScriptController::start(imported::auscope::API),
            receiver: audio_receiver,
            selected_device: None,
            selected_channels: None,
            selected_script_name: None,
            alert_message: None,
        }
    }

    pub fn devices(&self) -> &[AudioDevice] {
        self.receiver.list_audio_devices()
    }

    pub fn take_alert(&mut self) -> Option<String> {
        self.alert_message.take()
    }

    pub fn selected_script(&self) -> Option<&str> {
        self.selected_script_name.as_deref()
    }

    pub fn loaded_script_path(&self) -> Option<&PathBuf> {
        self.script.path()
    }

    pub fn audio(&self) -> &AudioBuffer {
        &self.buffer
    }

    pub fn audio_mut(&mut self) -> &mut AudioBuffer {
        &mut self.buffer
    }

    pub fn update_channel_selection(
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

        if let Err(e) = self
            .script
            .try_send(HostEvent::Connect(audio_device.name.clone()))
        {
            log::error!("Failed to send device connected event to runtime : {e}");
        }

        Ok(())
    }

    pub fn connect_to_audio_input(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        self.selected_device = Some(audio_device.clone());
        self.buffer.num_channels = channel_selection.count() as u32;
        self.update_channel_selection(channel_selection)
    }

    pub fn fetch_audio(&mut self) -> anyhow::Result<()> {
        self.receiver.process_audio_events()?;

        let mut audio = self.receiver.retrieve_audio_buffer();

        if self.buffer.num_channels != audio.num_channels {
            self.buffer = audio;
        } else {
            self.buffer.data.append(&mut audio.data);
        }

        Ok(())
    }

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

        if let Err(e) = self.script.try_send(HostEvent::Discover(
            self.devices().iter().map(|dev| dev.name.clone()).collect(),
        )) {
            log::error!("failed to send discovery event : {e}");
        }

        if self.selected_device.is_some() && self.selected_channels.is_some() {
            self.connect_to_audio_input(
                &self.selected_device.as_ref().unwrap().clone(),
                self.selected_channels.as_ref().unwrap().clone(),
            )?;
        }

        Ok(AppEvent::Continue)
    }

    pub fn process_script_events(&mut self) -> anyhow::Result<AppEvent> {
        while let Ok(script_event) = self.script.try_recv() {
            match script_event {
                ScriptEvent::Loaded => return Ok(AppEvent::ScriptLoaded),
                ScriptEvent::Log(request) => self.handle_lua_log_request(request),
                ScriptEvent::Connect(request) => self.handle_lua_connect_request(request)?,
                _ => (),
            }
        }

        Ok(AppEvent::Continue)
    }

    pub fn process_file_events(&mut self) -> anyhow::Result<AppEvent> {
        if self.script.was_script_modified()? && self.script.path().is_some() {
            self.load_script(self.script.path().unwrap().clone())
        } else {
            Ok(AppEvent::Continue)
        }
    }

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

        let device = self
            .devices()
            .iter()
            .find(|dev| dev.name == *device)
            .cloned();

        if let Some(device) = device {
            let channels = self
                .selected_channels
                .clone()
                .unwrap_or(AudioChannelSelection::Mono(0));
            self.connect_to_audio_input(&device, channels)?;
        }

        Ok(())
    }

    fn handle_lua_log_request(&mut self, request: LogApiEvent) {
        match request {
            LogApiEvent::Log(msg) => log::info!("{msg}"),
            LogApiEvent::Alert(msg) => self.alert_message = Some(msg),
        }
    }
}

// Pipes audio received from the remote into the provider.
// `RemoteAudioReceiver` receives audio and pushes into a
// consumer. In our case, we want to grab that audio
// through the `RemteAudioProvider`.
struct AudioPipe {
    sender: channel::Sender<AudioBuffer>,
}

impl AudioConsuming for AudioPipe {
    fn consume_audio_buffer(&mut self, buffer: AudioBuffer) -> anyhow::Result<()> {
        self.sender.try_send(buffer)?;
        Ok(())
    }
}

pub struct RemoteAudioProvider {
    interface: RemoteAudioReceiver<AudioPipe>,
    receiver: channel::Receiver<AudioBuffer>,
}

impl RemoteAudioProvider {
    pub fn new<Socket>(sockets: Sockets<Socket>) -> anyhow::Result<Self>
    where
        Socket: SocketInterface + 'static,
    {
        let (sender, receiver) = channel::bounded(16);
        let pipe = AudioPipe { sender };
        let interface = RemoteAudioReceiver::new(pipe, sockets)?;

        Ok(Self {
            interface,
            receiver,
        })
    }
}

impl AudioProviding for RemoteAudioProvider {
    fn retrieve_audio_buffer(&mut self) -> AudioBuffer {
        self.receiver.try_recv().unwrap_or_default()
    }
}

impl AudioInterface for RemoteAudioProvider {
    fn is_accessible(&self) -> bool {
        self.interface.is_accessible()
    }

    fn list_audio_devices(&self) -> &[AudioDevice] {
        self.interface.list_audio_devices()
    }

    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        self.interface
            .connect_to_audio_device(audio_device, channel_selection)
    }

    fn connected_audio_device(&self) -> Option<&AudioDeviceConnection> {
        self.interface.connected_audio_device()
    }

    fn process_audio_events(&mut self) -> anyhow::Result<()> {
        self.interface.process_audio_events()
    }
}
