#![allow(non_camel_case_types)]

use super::*;
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
            is_active: false,
        }
    }
}

impl AudioProviding for FfiAudioReceiver {
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
        self.selected_channels = channel_selection.to_vec();

        let Some(i) = self.sources.iter().position(|s| s == audio_device) else {
            anyhow::bail!("Audio device not found : {}", audio_device.name);
        };

        self.is_active = true;
        self.selected_source = Some(i);
        Ok(())
    }

    fn try_fetch_audio(&mut self) -> anyhow::Result<AudioBuffer> {
        Ok(self.receiver.try_recv()?)
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
                channels: source.num_channels as usize,
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
    deinterleave_data: *const f32,
    num_samples: c_uint,
    num_channels: c_uint,
) {
    if ctx.is_null() {
        return;
    }

    let num_samples = num_samples as usize;
    let num_channels = num_channels as usize;

    let audio = &*(ctx as *mut FfiAudioReceiver);
    if !audio.is_active {
        return;
    }

    let Some(source_index) = audio.selected_source else {
        return;
    };

    let source_str = match CStr::from_ptr(source_name).to_str() {
        Ok(source) => source,
        Err(e) => {
            log::error!("Failed to parse audio source name with error : {e}");
            return;
        }
    };

    if audio.sources[source_index].name != source_str {
        return;
    }

    let data = slice::from_raw_parts(deinterleave_data, num_channels * num_samples);
    let mut buffer = vec![vec![0.; num_samples]; num_channels];
    for chan in 0..num_channels {
        if !audio.selected_channels.contains(&chan) {
            continue;
        }

        let channel_data = slice::from_raw_parts(&data[chan], num_samples);
        buffer[chan][..num_samples].copy_from_slice(&channel_data[..num_samples]);
    }

    if let Err(e) = audio.sender.try_send(buffer) {
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
