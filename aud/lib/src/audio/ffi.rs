#![allow(non_camel_case_types)]

use super::*;
use crossbeam::channel::{Receiver, Sender, TryRecvError};
use std::{
    ffi::{c_char, c_uint, c_void, CStr},
    slice,
};

pub struct FfiAudioReceiver {
    sender: Sender<AudioBuffer>,
    receiver: Receiver<AudioBuffer>,
    sources: Vec<AudioDevice>,
    selected_source: Option<usize>,
    selected_channels: Vec<usize>,
    audio: AudioBuffer,
    is_active: bool,
}

impl Default for FfiAudioReceiver {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(100);

        Self {
            sender,
            receiver,
            sources: vec![],
            selected_source: None,
            selected_channels: vec![],
            audio: AudioBuffer::default(),
            is_active: false,
        }
    }
}

impl AudioProviding for FfiAudioReceiver {
    fn is_accessible(&self) -> bool {
        self.selected_source.is_some()
    }

    fn list_audio_devices(&self) -> &[AudioDevice] {
        self.sources.as_slice()
    }

    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        if channel_selection.is_valid_for_device(audio_device) {
            log::error!("Invalid selection : {channel_selection:#?} for : {audio_device:#?}");
            return Ok(());
        }

        let Some(i) = self.sources.iter().position(|s| s == audio_device) else {
            anyhow::bail!("Audio device not found : {}", audio_device.name);
        };

        self.is_active = true;
        self.selected_source = Some(i);
        self.selected_channels = channel_selection.to_vec();
        Ok(())
    }

    fn retrieve_audio_buffer(&mut self) -> AudioBuffer {
        std::mem::take(&mut self.audio)
    }

    fn process_audio_events(&mut self) -> anyhow::Result<()> {
        match self.receiver.try_recv() {
            Ok(mut audio) => {
                self.audio.num_channels = audio.num_channels;
                self.audio.data.append(&mut audio.data)
            }
            Err(TryRecvError::Empty) => (),
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }
}

#[repr(C)]
pub struct aud_audio_device_t {
    name: *const c_char,
    num_channels: c_uint,
}

/// Create an Audio Stream instance.
///
/// The library consumer should
/// create, setup the sources,
/// then pass a pointer to this
/// stream to any `aud` audio
/// consumer
#[no_mangle]
pub extern "C" fn aud_audio_stream_create() -> *mut c_void {
    let audio = Box::<FfiAudioReceiver>::default();
    Box::into_raw(audio) as *mut _
}

/// # Safety
///
/// Not thread-safe, needs to be called from the same
/// thread that calls `create()`
#[no_mangle]
pub unsafe extern "C" fn aud_audio_stream_set_sources(
    ctx: *mut c_void,
    sources: *const aud_audio_device_t,
    num_sources: c_uint,
) {
    if ctx.is_null() {
        return;
    }

    let num_sources = num_sources as usize;

    let audio = &mut *(ctx as *mut FfiAudioReceiver);
    audio.sources.clear();

    for source in slice::from_raw_parts(sources, num_sources).iter() {
        match CStr::from_ptr(source.name).to_str() {
            Ok(name) => audio.sources.push(AudioDevice {
                name: name.to_owned(),
                num_channels: source.num_channels as usize,
            }),
            Err(e) => log::error!("Failed to add audio source names with error : {e}"),
        }
    }
}

/// # Safety
///
/// This is thread-safe.
///
/// The caller must supply the name of this source
#[no_mangle]
pub unsafe extern "C" fn aud_audio_stream_push(
    ctx: *mut c_void,
    source_name: *mut c_char,
    interleaved_buffer: *const f32,
    num_samples: c_uint,
    num_channels: c_uint,
) {
    if ctx.is_null() {
        return;
    }

    let receiver = &*(ctx as *mut FfiAudioReceiver);
    if !receiver.is_active {
        return;
    }

    let Some(source_index) = receiver.selected_source else {
        return;
    };

    let source_str = match CStr::from_ptr(source_name).to_str() {
        Ok(source) => source,
        Err(e) => {
            log::error!("Failed to parse audio source name with error : {e}");
            return;
        }
    };

    if receiver.sources[source_index].name != source_str {
        return;
    }

    let data = slice::from_raw_parts(interleaved_buffer, (num_channels * num_samples) as usize);
    let num_requested_channels = receiver.selected_channels.len() as u32;
    let mut write_chan = 0;
    let mut buffer = AudioBuffer::new(num_samples, num_requested_channels);
    for (chan, frame) in data.chunks(num_channels as usize).enumerate() {
        if !receiver.selected_channels.contains(&chan) {
            continue;
        }

        for (sample, value) in frame.iter().enumerate() {
            buffer.data[write_chan * buffer.num_channels as usize + sample] = *value;
        }

        write_chan += 1;
    }

    if let Err(e) = receiver.sender.try_send(buffer) {
        log::error!("Failed to send the audio buffer received from the FFI : {e}");
    }
}

/// Clean up the Audio Stream
/// instance. Ensure the validity
/// of the pointer, it must have
/// been create by a `create`
#[no_mangle]
pub extern "C" fn aud_audio_stream_destroy(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }

    unsafe {
        let _sender: Box<FfiAudioReceiver> = Box::from_raw(ctx as *mut _);
    }
}
