use super::*;
use std::{ffi::c_void, slice};

pub struct FfiAudioReceiver {
    sender: Sender<AudioBuffer>,
    receiver: Receiver<AudioBuffer>,
    device_names: Vec<String>,
    selected_device: Option<String>,
    is_active: bool,
}

impl Default for FfiAudioReceiver {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(100);

        Self {
            sender,
            receiver,
            device_names: vec![],
            selected_device: None,
            is_active: false,
        }
    }
}

impl AudioReceiving for FfiAudioReceiver {
    fn open_stream(&mut self) -> anyhow::Result<()> {
        self.is_active = true;
        Ok(())
    }

    fn list_devices(&self) -> anyhow::Result<Vec<String>> {
        Ok(self.device_names.clone())
    }

    fn select_device(&mut self, device_name: &str) -> anyhow::Result<()> {
        if self.device_names.iter().any(|name| name == device_name) {
            anyhow::bail!("Audio device not found : {device_name}");
        }

        self.selected_device = Some(device_name.to_owned());
        Ok(())
    }

    fn try_receive_audio(&mut self) -> anyhow::Result<AudioBuffer> {
        Ok(self.receiver.try_recv()?)
    }
}

#[no_mangle]
pub extern "C" fn aud_audio_stream_create() -> *mut c_void {
    let recv = Box::<FfiAudioReceiver>::default();
    Box::into_raw(recv) as *mut _
}

/// # Safety
///
/// This is thread-safe.
#[no_mangle]
pub unsafe extern "C" fn aud_audio_stream_push(
    ctx: *mut c_void,
    deinterleave_data: *const f32,
    num_channels: usize,
    num_samples: usize,
) {
    let recv = &*(ctx as *mut FfiAudioReceiver);
    if !recv.is_active {
        return;
    }

    let data = slice::from_raw_parts(deinterleave_data, num_channels * num_samples);
    let mut buffer = vec![vec![0.; num_samples]; num_channels];
    for chan in 0..num_channels {
        let channel_data = slice::from_raw_parts(&data[chan], num_samples);
        buffer[chan][..num_samples].copy_from_slice(&channel_data[..num_samples]);
    }

    if let Err(e) = recv.sender.try_send(buffer) {
        log::error!("Failed to send the audio buffer received from the FFI : {e}");
    }
}

#[no_mangle]
pub extern "C" fn aud_audio_stream_destroy(ctx: *mut c_void) {
    unsafe {
        let _sender: Box<FfiAudioReceiver> = Box::from_raw(ctx as *mut _);
    }
}
