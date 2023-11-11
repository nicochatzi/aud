#![allow(non_camel_case_types)]

use crate::comms::Sockets;

use super::*;
use crossbeam::channel::{Receiver, Sender, TryRecvError};
use std::{
    ffi::{c_char, c_uint, CStr},
    net::UdpSocket,
    slice,
};

struct CachingAudioProvider {
    receiver: Receiver<AudioBuffer>,
    sources: Vec<AudioDevice>,
    selected_source: Option<usize>,
    selected_channels: Vec<usize>,
    connected_device: Option<AudioDeviceConnection>,
    audio: AudioBuffer,
    is_active: bool,
}

impl CachingAudioProvider {
    fn new(receiver: Receiver<AudioBuffer>, sources: Vec<AudioDevice>) -> Self {
        Self {
            receiver,
            sources,
            selected_source: None,
            selected_channels: vec![],
            connected_device: None,
            audio: AudioBuffer::default(),
            is_active: false,
        }
    }
}

impl AudioInterface for CachingAudioProvider {
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
        if !audio_device.supports_channels(&channel_selection) {
            log::error!("Invalid selection : {channel_selection:#?} for : {audio_device:#?}");
            return Ok(());
        }

        let Some(i) = self.sources.iter().position(|s| s == audio_device) else {
            anyhow::bail!("Audio device not found : {}", audio_device.name);
        };

        self.is_active = true;
        self.selected_source = Some(i);
        self.selected_channels = channel_selection.as_vec();
        Ok(())
    }

    fn connected_audio_device(&self) -> Option<&AudioDeviceConnection> {
        self.connected_device.as_ref()
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

impl AudioProviding for CachingAudioProvider {
    fn retrieve_audio_buffer(&mut self) -> AudioBuffer {
        std::mem::take(&mut self.audio)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
enum FfiAudioTransmitterResult {
    NoError = 0,
    AudioPushed,
    NoSourceCurrentlySelected,
    OtherSourceSelected,

    FailedToConnectToSocket,
    FailedToParseInputSocket,
    FailedToParseOutputAddress,
    InvalidSourcePointer,
    FailedToParseAudioSource,
    InvalidTransmitterPointer,
}

struct FfiAudioTransmitter {
    transmitter: RemoteAudioTransmitter<CachingAudioProvider>,
    sender: Sender<AudioBuffer>,
}

#[repr(C)]
struct aud_transmitter_initializer_t {
    input_socket: aud_udp_socket_t,
    output_socket: aud_udp_socket_t,
    sources: *const aud_audio_device_t,
    num_sources: c_uint,
}

impl aud_transmitter_initializer_t {
    fn parse_sources(&self) -> Result<Vec<AudioDevice>, FfiAudioTransmitterResult> {
        if self.sources.is_null() {
            return Err(FfiAudioTransmitterResult::InvalidSourcePointer);
        }

        let sources_slice =
            unsafe { slice::from_raw_parts(self.sources, self.num_sources as usize) };
        let mut devices = Vec::with_capacity(self.num_sources as usize);

        for source in sources_slice {
            let name_cstr = unsafe { CStr::from_ptr(source.name) };
            match name_cstr.to_str() {
                Ok(name) => devices.push(AudioDevice {
                    name: name.to_owned(),
                    num_channels: source.num_channels as usize,
                }),
                Err(_) => return Err(FfiAudioTransmitterResult::FailedToParseAudioSource),
            }
        }

        Ok(devices)
    }
}

#[repr(C)]
struct aud_udp_socket_t {
    port_name: *const c_char,
    port_name_length: c_uint,
}

impl aud_udp_socket_t {
    fn parse(&self) -> Result<String, FfiAudioTransmitterResult> {
        if self.port_name.is_null() {
            return Err(FfiAudioTransmitterResult::FailedToParseInputSocket);
        }

        let cstr = unsafe { CStr::from_ptr(self.port_name) };
        Ok(cstr.to_string_lossy().into_owned())
    }
}

#[repr(C)]
struct aud_audio_device_t {
    name: *const c_char,
    num_channels: c_uint,
}

/// Creates an audio transmitter based on the provided initializer.
///
/// This function attempts to create a `FfiAudioTransmitter` using the information
/// provided in the `aud_transmitter_initializer_t` struct. It sets up the necessary
/// resources such as input/output sockets and audio sources, and initializes a new
/// audio transmitter which is returned through an out parameter.
///
/// # Parameters
/// - `initializer`: A struct containing the necessary information to initialize
///   the audio transmitter.
/// - `transmitter_out`: A pointer to a pointer where the address of the newly
///   created `FfiAudioTransmitter` will be stored. It's the caller's responsibility
///   to later free this memory.
///
/// # Returns
/// - A `FfiAudioTransmitterResult` enum value indicating the result of the operation.
///   `FfiAudioTransmitterResult::NoError` indicates success, while other values
///   indicate specific errors that occurred.
///
/// # Safety
/// This function is `unsafe` as it involves dereferencing raw pointers provided
/// by the caller, and because it returns a heap-allocated object to the caller
/// which must be properly freed to avoid a memory leak.
///
/// # Example (C code)
/// ```c
/// aud_transmitter_initializer_t initializer = { ... };
/// FfiAudioTransmitter* transmitter = NULL;
/// FfiAudioTransmitterResult retval = aud_audio_transmitter_create(initializer, &transmitter);
/// if (retval == NoError) {
///     // Use transmitter...
/// } else {
///     // Handle error...
/// }
/// // Eventually free the transmitter using `aud_audio_transmitter_destroy(&transmitter)`
/// ```
#[no_mangle]
extern "C" fn aud_audio_transmitter_create(
    initializer: aud_transmitter_initializer_t,
    transmitter_out: *mut *mut FfiAudioTransmitter,
) -> FfiAudioTransmitterResult {
    let (sender, receiver) = crossbeam::channel::bounded(100);

    let Ok(sources) = initializer.parse_sources() else {
        return FfiAudioTransmitterResult::FailedToParseAudioSource;
    };

    let Ok(socket) = initializer.input_socket.parse() else {
        return FfiAudioTransmitterResult::FailedToParseInputSocket;
    };

    let Ok(target) = initializer.output_socket.parse() else {
        return FfiAudioTransmitterResult::FailedToParseOutputAddress;
    };

    let (Ok(socket), Ok(target)) = (UdpSocket::bind(socket), target.parse()) else {
        return FfiAudioTransmitterResult::FailedToConnectToSocket;
    };

    let provider = CachingAudioProvider::new(receiver, sources);
    let sockets = Sockets::<UdpSocket> { socket, target };

    let Ok(transmitter) = RemoteAudioTransmitter::<CachingAudioProvider>::new(provider, sockets)
    else {
        return FfiAudioTransmitterResult::FailedToConnectToSocket;
    };

    let ffi = FfiAudioTransmitter {
        transmitter,
        sender,
    };

    if transmitter_out.is_null() {
        return FfiAudioTransmitterResult::InvalidTransmitterPointer;
    }

    unsafe { *transmitter_out = Box::into_raw(Box::new(ffi)) };
    FfiAudioTransmitterResult::NoError
}

/// Pushes an interleaved audio buffer to the specified transmitter.
///
/// This function takes a buffer of audio data, extracts the specified channels,
/// and sends the resulting buffer to the transmitter for further processing.
///
/// Note that if the remote has not selected this device the audio
/// will not actually be pushed and this function will be reasonably
/// cheap. Suitable for Debug or RelWithDebInfo-style builds.
///
/// # Parameters
/// - `transmitter`: A pointer to the `FfiAudioTransmitter` instance.
/// - `source_name`: A pointer to a null-terminated string representing the name of the audio source.
/// - `interleaved_buffer`: A pointer to the buffer containing interleaved audio data.
/// - `num_frames`: The number of frames in the audio buffer.
/// - `num_channels`: The total number of channels in the audio buffer.
///
/// # Returns
/// - `FfiAudioTransmitterResult::AudioPushed` on success.
/// - `FfiAudioTransmitterResult::InvalidTransmitterPointer` if the `transmitter` pointer is null.
/// - `FfiAudioTransmitterResult::NoSourceCurrentlySelected` if no audio source is selected or the transmitter is not accessible.
/// - `FfiAudioTransmitterResult::InvalidSourcePointer` if the `source_name` pointer could not be converted to a string.
/// - `FfiAudioTransmitterResult::OtherSourceSelected` if a different audio source is currently selected.
///
/// # Safety
/// This function is unsafe as it operates on raw pointers from FFI, and requires the caller to ensure that the provided pointers are valid,
/// the audio buffer contains the expected number of frames and channels, and the `transmitter` has been properly initialized.
///
/// # Example (C code)
/// ```c
/// // Assuming transmitter was previously created with aud_audio_transmitter_create...
/// FfiAudioTransmitterResult retval = aud_audio_transmitter_push(transmitter, "My Audio Source", interleaved_buffer, num_frames, num_channels);
/// if (retval != FfiAudioTransmitterResult::AudioPushed) {
///    // Handle error..
///    return;
/// }
/// ```
///
#[no_mangle]
unsafe extern "C" fn aud_audio_transmitter_push(
    transmitter: *mut FfiAudioTransmitter,
    source_name: *mut c_char,
    interleaved_buffer: *const f32,
    num_frames: c_uint,
    num_channels: c_uint,
) -> FfiAudioTransmitterResult {
    if transmitter.is_null() {
        return FfiAudioTransmitterResult::InvalidTransmitterPointer;
    }

    let tx = &*transmitter;
    if !(tx.transmitter.is_accessible() && tx.transmitter.connected_audio_device().is_some()) {
        return FfiAudioTransmitterResult::NoSourceCurrentlySelected;
    }

    let connected_audio_device = tx.transmitter.connected_audio_device();
    let connection = connected_audio_device.as_ref().unwrap();

    let Ok(source_name) = CStr::from_ptr(source_name).to_str() else {
        log::error!("Failed to parse audio source name");
        return FfiAudioTransmitterResult::InvalidSourcePointer;
    };

    if connection.device.name != source_name {
        return FfiAudioTransmitterResult::OtherSourceSelected;
    }

    let data = slice::from_raw_parts(interleaved_buffer, (num_channels * num_frames) as usize);
    let selected_channels = connection.channels.as_vec();
    let mut buffer = AudioBuffer::with_frames(num_frames, selected_channels.len() as u32);

    for frame in 0..num_frames as usize {
        for (write_chan, &chan) in selected_channels.iter().enumerate() {
            let read_index = frame * num_channels as usize + chan;
            let write_index = frame * selected_channels.len() + write_chan;
            buffer.data[write_index] = data[read_index];
        }
    }

    if let Err(e) = tx.sender.try_send(buffer) {
        log::error!(
            "Failed to send the audio buffer received from the FFI: {:?}",
            e
        );
    }

    FfiAudioTransmitterResult::AudioPushed
}

/// Frees the resources associated with a previously created `FfiAudioTransmitter`.
///
/// This function is responsible for cleaning up the resources associated with a
/// `FfiAudioTransmitter` instance. It should be called once for every successful
/// call to `aud_audio_transmitter_create`.
///
/// # Parameters
/// - `transmitter`: A pointer to the `FfiAudioTransmitter` to be destroyed.
///
/// # Safety
/// This function is `unsafe` because it performs deallocation and because improper
/// use can lead to memory leaks or other undefined behavior (e.g., if called
/// more than once with the same pointer or if the pointer was not previously
/// returned by `aud_audio_transmitter_create`).
///
/// # Example (C code)
/// ```c
/// // Assuming transmitter was previously created with aud_audio_transmitter_create...
/// aud_audio_transmitter_destroy(transmitter);
/// ```
#[no_mangle]
extern "C" fn aud_audio_transmitter_destroy(transmitter: *mut FfiAudioTransmitter) {
    // Ensuring that the provided pointer is not null before proceeding
    if !transmitter.is_null() {
        unsafe {
            // Dropping the FfiAudioTransmitter, which will also drop its owned resources
            drop(Box::from_raw(transmitter));
        }
    }
}
