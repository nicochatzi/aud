use std::collections::HashSet;

use super::*;
use cpal::{traits::*, FromSample, Sample, SizedSample};
use crossbeam::channel::{Receiver, Sender};

pub struct HostAudioInput {
    host: cpal::Host,
    sender: Sender<AudioBuffer>,
    receiver: Receiver<AudioBuffer>,
    stream: AudioStream,
    devices: Vec<AudioDevice>,
    connected_device: Option<AudioDeviceConnection>,
    audio: AudioBuffer,
}

impl Default for HostAudioInput {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(128);
        let host = cpal::default_host();
        let devices = match host.input_devices() {
            Ok(devices) => devices.filter_map(AudioDevice::try_from_input).collect(),
            Err(err) => {
                log::error!("Failed to get input devices: {}", err);
                vec![]
            }
        };

        Self {
            stream: AudioStream::default(),
            sender,
            receiver,
            devices,
            audio: AudioBuffer::default(),
            connected_device: None,
            host,
        }
    }
}

impl AudioInterface for HostAudioInput {
    fn is_accessible(&self) -> bool {
        self.stream.is_open()
    }

    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        if !audio_device.supports_channels(&channel_selection) {
            log::error!("Invalid output selection : {channel_selection:?} for : {audio_device:#?}");
            return Ok(());
        }

        self.stream = self
            .host
            .input_devices()?
            .find(|device| device.name().ok().as_deref() == Some(&audio_device.name))
            .map(|device| {
                AudioStream::open_input(self.sender.clone(), &device, channel_selection.clone())
            })
            .ok_or_else(|| anyhow::anyhow!("No audio input device selected"))??;

        self.connected_device = Some(AudioDeviceConnection {
            device: audio_device.to_owned(),
            channels: channel_selection,
            sample_rate: self.stream.config.as_ref().unwrap().sample_rate.0,
        });

        Ok(())
    }

    fn connected_audio_device(&self) -> Option<&AudioDeviceConnection> {
        self.connected_device.as_ref()
    }

    fn list_audio_devices(&self) -> &[AudioDevice] {
        self.devices.as_slice()
    }

    fn process_audio_events(&mut self) -> anyhow::Result<()> {
        for mut buffer in self.receiver.try_iter() {
            if buffer.num_channels != self.audio.num_channels {
                self.audio = buffer;
            } else {
                self.audio.data.append(&mut buffer.data);
            }
        }
        Ok(())
    }
}

impl AudioProviding for HostAudioInput {
    fn retrieve_audio_buffer(&mut self) -> AudioBuffer {
        std::mem::take(&mut self.audio)
    }
}

pub struct HostAudioOutput {
    host: cpal::Host,
    sender: Sender<AudioBuffer>,
    receiver: Receiver<AudioBuffer>,
    stream: AudioStream,
    devices: Vec<AudioDevice>,
    connected_device: Option<AudioDeviceConnection>,
}

impl Default for HostAudioOutput {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(16);
        let host = cpal::default_host();
        let devices = match host.output_devices() {
            Ok(devices) => devices.filter_map(AudioDevice::try_from_output).collect(),
            Err(err) => {
                log::error!("Failed to get output devices: {}", err);
                vec![]
            }
        };

        Self {
            stream: AudioStream::default(),
            sender,
            receiver,
            devices,
            connected_device: None,
            host,
        }
    }
}

impl AudioInterface for HostAudioOutput {
    fn is_accessible(&self) -> bool {
        self.stream.is_open()
    }

    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        if !audio_device.supports_channels(&channel_selection) {
            log::error!("Invalid output selection : {channel_selection:?} for : {audio_device:#?}");
            return Ok(());
        }

        self.stream = self
            .host
            .output_devices()?
            .find(|device| device.name().ok().as_deref() == Some(&audio_device.name))
            .map(|device| {
                AudioStream::open_output(self.receiver.clone(), &device, channel_selection.clone())
            })
            .ok_or_else(|| anyhow::anyhow!("No audio output device selected"))??;

        self.connected_device = Some(AudioDeviceConnection {
            device: audio_device.to_owned(),
            channels: channel_selection,
            sample_rate: self.stream.config.as_ref().unwrap().sample_rate.0,
        });
        Ok(())
    }

    fn connected_audio_device(&self) -> Option<&AudioDeviceConnection> {
        self.connected_device.as_ref()
    }

    fn list_audio_devices(&self) -> &[AudioDevice] {
        self.devices.as_slice()
    }

    fn process_audio_events(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl AudioConsuming for HostAudioOutput {
    fn consume_audio_buffer(&mut self, buffer: AudioBuffer) -> anyhow::Result<()> {
        Ok(self.sender.try_send(buffer)?)
    }
}

impl AudioDevice {
    fn try_from_output(device: cpal::Device) -> Option<Self> {
        Some(Self::try_from_config(
            device.name().ok()?,
            device.default_output_config().ok()?,
        ))
    }

    fn try_from_input(device: cpal::Device) -> Option<Self> {
        Some(Self::try_from_config(
            device.name().ok()?,
            device.default_input_config().ok()?,
        ))
    }

    fn try_from_config(name: String, config: cpal::SupportedStreamConfig) -> Self {
        Self {
            name,
            num_channels: config.channels() as usize,
        }
    }
}

#[derive(Default)]
struct AudioStream {
    stream: Option<cpal::Stream>,
    config: Option<cpal::StreamConfig>,
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
        }
    }
}

impl AudioStream {
    fn is_open(&self) -> bool {
        self.stream.is_some()
    }

    fn open_input(
        tx: Sender<AudioBuffer>,
        dev: &cpal::Device,
        sel: AudioChannelSelection,
    ) -> anyhow::Result<Self> {
        let (config, sample_format) = setup_preferred_stream_config(dev.default_input_config()?);

        let stream = match sample_format {
            cpal::SampleFormat::I8 => read::<i8>(tx, dev, &config, sel),
            cpal::SampleFormat::I16 => read::<i16>(tx, dev, &config, sel),
            cpal::SampleFormat::I32 => read::<i32>(tx, dev, &config, sel),
            cpal::SampleFormat::I64 => read::<i64>(tx, dev, &config, sel),
            cpal::SampleFormat::U8 => read::<u8>(tx, dev, &config, sel),
            cpal::SampleFormat::U16 => read::<u16>(tx, dev, &config, sel),
            cpal::SampleFormat::U32 => read::<u32>(tx, dev, &config, sel),
            cpal::SampleFormat::U64 => read::<u64>(tx, dev, &config, sel),
            cpal::SampleFormat::F32 => read::<f32>(tx, dev, &config, sel),
            cpal::SampleFormat::F64 => read::<f64>(tx, dev, &config, sel),
            sample_format => anyhow::bail!("Unsupported sample format '{sample_format}'"),
        }?;

        Ok(Self {
            stream: Some(stream),
            config: Some(config),
        })
    }

    fn open_output(
        rx: Receiver<AudioBuffer>,
        dev: &cpal::Device,
        sel: AudioChannelSelection,
    ) -> anyhow::Result<Self> {
        let (config, sample_format) = setup_preferred_stream_config(dev.default_input_config()?);

        let stream = match sample_format {
            cpal::SampleFormat::I8 => write::<i8>(rx, dev, &config, sel),
            cpal::SampleFormat::I16 => write::<i16>(rx, dev, &config, sel),
            cpal::SampleFormat::I32 => write::<i32>(rx, dev, &config, sel),
            cpal::SampleFormat::I64 => write::<i64>(rx, dev, &config, sel),
            cpal::SampleFormat::U8 => write::<u8>(rx, dev, &config, sel),
            cpal::SampleFormat::U16 => write::<u16>(rx, dev, &config, sel),
            cpal::SampleFormat::U32 => write::<u32>(rx, dev, &config, sel),
            cpal::SampleFormat::U64 => write::<u64>(rx, dev, &config, sel),
            cpal::SampleFormat::F32 => write::<f32>(rx, dev, &config, sel),
            cpal::SampleFormat::F64 => write::<f64>(rx, dev, &config, sel),
            sample_format => anyhow::bail!("Unsupported sample format '{sample_format}'"),
        }?;

        Ok(Self {
            stream: Some(stream),
            config: Some(config),
        })
    }
}

fn setup_preferred_stream_config(
    default_config: cpal::SupportedStreamConfig,
) -> (cpal::StreamConfig, cpal::SampleFormat) {
    let sample_format = default_config.sample_format();

    let buffer_size = match *default_config.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => [512, 1024, 256]
            .iter()
            .find(|&&value| (min..max).contains(&value))
            .map(|&value| cpal::BufferSize::Fixed(value)),
        cpal::SupportedBufferSize::Unknown => {
            log::warn!("CPAL failed to find a buffer size range");
            None
        }
    };

    let mut config: cpal::StreamConfig = default_config.into();
    if let Some(buffer_size) = buffer_size {
        config.buffer_size = buffer_size;
    }

    (config, sample_format)
}

fn read<T>(
    sender: Sender<AudioBuffer>,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    selection: AudioChannelSelection,
) -> anyhow::Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32> + 'static,
{
    let enqueue_audio_input_data = make_audio_buffer_enqueueing_function::<T>(
        sender,
        config.channels as usize,
        selection.as_vec(),
    );

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| enqueue_audio_input_data(data),
        move |e| log::error!("an error occurred in audio input stream: {e}"),
        None,
    )?;

    stream.play()?;
    log::trace!("started audio input stream");
    Ok(stream)
}

fn write<T>(
    receiver: Receiver<AudioBuffer>,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    selection: AudioChannelSelection,
) -> anyhow::Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32> + 'static,
{
    let dequeue_audio_buffers_into_host =
        make_audio_dequeuing_function::<T>(receiver, config.channels as usize, selection.as_vec());

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _| dequeue_audio_buffers_into_host(data),
        move |e| log::error!("an error occurred in the audio output stream: {e}"),
        None,
    )?;

    stream.play()?;
    log::trace!("started audio output stream");
    Ok(stream)
}

fn make_audio_buffer_enqueueing_function<T>(
    sender: Sender<AudioBuffer>,
    num_input_channels: usize,
    selected_channels: Vec<usize>,
) -> impl Fn(&[T])
where
    T: SizedSample + FromSample<f32>,
{
    let num_requested_channels = selected_channels.len();

    move |audio_buffer: &[T]| {
        let num_frames = audio_buffer.len() / num_input_channels;
        let mut buffer = AudioBuffer::with_length(
            (num_frames * num_requested_channels) as u32,
            num_requested_channels as u32,
        );

        for (chan_idx, &read_chan) in selected_channels.iter().enumerate() {
            for frame in 0..num_frames {
                let read_index = frame * num_input_channels + read_chan;
                let write_index = frame * num_requested_channels + chan_idx;
                buffer.data[write_index] = audio_buffer[read_index]
                    .to_float_sample()
                    .to_sample::<f32>();
            }
        }

        if let Err(e) = sender.try_send(buffer) {
            log::error!("failed to push audio out of CPAL: {}", e);
        }
    }
}

fn make_audio_dequeuing_function<T>(
    receiver: Receiver<AudioBuffer>,
    total_num_channels: usize,
    channels: Vec<usize>,
) -> impl Fn(&mut [T])
where
    T: SizedSample + FromSample<f32>,
{
    let channels_set: HashSet<usize> = channels.iter().cloned().collect();

    move |audio_buffer: &mut [T]| {
        let num_frames_to_provide = audio_buffer.len() / total_num_channels;
        let mut received_buffers = Vec::with_capacity(4);
        let mut num_frames_received = 0;
        while let Ok(received_buffer) = receiver.try_recv() {
            num_frames_received += received_buffer.num_frames();
            received_buffers.push(received_buffer);
            if num_frames_received >= num_frames_to_provide {
                break;
            }
        }

        let buffer = AudioBuffer::from_buffers(received_buffers);
        let buffer_num_channels = buffer.num_channels as usize;
        if buffer_num_channels == 0 {
            return;
        }

        let buffer_num_frames = buffer.data.len() / buffer_num_channels;

        for (frame_idx, frame) in audio_buffer.chunks_mut(total_num_channels).enumerate() {
            for (write_chan, output) in frame.iter_mut().enumerate().take(total_num_channels) {
                if !channels_set.contains(&write_chan) {
                    continue;
                }

                let read_chan = if buffer_num_channels == 1 {
                    0
                } else {
                    write_chan.min(buffer_num_channels - 1)
                };

                let value = if frame_idx < buffer_num_frames {
                    buffer.data[frame_idx * buffer_num_channels + read_chan]
                } else {
                    0.0
                };

                *output = T::from_sample(value);
            }
        }
    }
}

#[cfg(feature = "bench")]
pub fn test_make_audio_buffer_enqueueing_function(
    sender: Sender<AudioBuffer>,
    num_input_channels: usize,
    selected_channels: Vec<usize>,
) -> impl Fn(&[f32]) {
    make_audio_buffer_enqueueing_function::<f32>(sender, num_input_channels, selected_channels)
}

#[cfg(feature = "bench")]
pub fn test_make_audio_dequeing_function(
    receiver: Receiver<AudioBuffer>,
    total_num_channels: usize,
    channels: Vec<usize>,
) -> impl Fn(&mut [f32]) {
    make_audio_dequeuing_function::<f32>(receiver, total_num_channels, channels)
}

#[cfg(test)]
mod test {
    use super::*;

    fn assign_channel_index_to_each_sample(buffer: &mut AudioBuffer) {
        for frame in buffer.data.chunks_mut(buffer.num_channels as usize) {
            for (chan, value) in frame.iter_mut().enumerate() {
                *value = chan as f32;
            }
        }
    }

    #[test]
    fn test_can_assign_channel_index_to_each_sample() {
        for (num_frames, num_channels) in [(128, 4), (32, 2), (512, 16)] {
            let mut buffer = AudioBuffer::with_frames(num_frames as u32, num_channels as u32);
            assign_channel_index_to_each_sample(&mut buffer);
            let expected_data: Vec<f32> = (0..num_channels as u32)
                .cycle()
                .take(num_channels as usize * num_frames as usize)
                .map(|i| i as f32)
                .collect();
            assert_eq!(buffer.data, expected_data);
        }
    }

    #[test]
    fn can_enqueue_audio_when_a_mono_channel_is_selected() {
        const MAX_NUM_CHANNELS: usize = 10;
        for num_channels in 1..MAX_NUM_CHANNELS {
            for selected_channel in 0..num_channels {
                let (sender, receiver) = crossbeam::channel::unbounded();

                let num_frames = 128;
                let channels = AudioChannelSelection::Mono(selected_channel);
                let process = make_audio_buffer_enqueueing_function::<f32>(
                    sender,
                    num_channels,
                    channels.as_vec(),
                );
                let mut buffer = AudioBuffer::with_frames(num_frames, num_channels as u32);
                assign_channel_index_to_each_sample(&mut buffer);

                process(&buffer.data);
                let messages: Vec<AudioBuffer> = receiver.try_iter().collect();
                assert_eq!(messages.len(), 1);
                assert_eq!(messages[0].num_channels, 1);
                assert!(messages[0]
                    .data
                    .iter()
                    .all(|sample| *sample == selected_channel as f32));
            }
        }
    }

    #[test]
    fn can_enqueue_audio_when_multiple_channels_are_selected() {
        const MAX_NUM_CHANNELS: usize = 10;
        for num_channels in 2..MAX_NUM_CHANNELS {
            for start in 0..num_channels - 1 {
                for end in start + 1..num_channels {
                    let (sender, receiver) = crossbeam::channel::unbounded();

                    let num_frames = 128;
                    let channels = AudioChannelSelection::Range(start..end).as_vec();
                    let process = make_audio_buffer_enqueueing_function::<f32>(
                        sender,
                        num_channels,
                        channels.clone(),
                    );

                    let mut buffer = AudioBuffer::with_frames(num_frames, num_channels as u32);
                    assign_channel_index_to_each_sample(&mut buffer);

                    process(&buffer.data);
                    let messages: Vec<AudioBuffer> = receiver.try_iter().collect();
                    assert_eq!(messages.len(), 1);
                    assert_eq!(messages[0].num_channels, channels.len() as u32);

                    for (idx, sample) in messages[0].data.iter().enumerate() {
                        let channel_idx = channels[idx % channels.len()];
                        let expected_value = channel_idx as f32;
                        assert_eq!(*sample, expected_value, "Sample index: {}", idx);
                    }
                }
            }
        }
    }

    #[test]
    fn can_dequeue_audio_into_an_output_buffer_for_a_mono_channel() {
        const MAX_NUM_CHANNELS: usize = 10;
        for num_channels in 1..MAX_NUM_CHANNELS {
            for selected_channel in 0..num_channels {
                let (sender, receiver) = crossbeam::channel::unbounded();

                let num_frames = 128;
                let channels = AudioChannelSelection::Mono(selected_channel).as_vec();
                let process =
                    make_audio_dequeuing_function(receiver, num_channels, channels.clone());

                let mut buffer_sent = AudioBuffer::with_frames(num_frames, num_channels as u32);
                assign_channel_index_to_each_sample(&mut buffer_sent);
                sender.send(buffer_sent.clone()).unwrap();

                let mut output_buffer = vec![0.0; num_frames as usize * num_channels];
                process(&mut output_buffer);

                for (frame_idx, frame) in output_buffer.chunks(num_channels).enumerate() {
                    for (chan_idx, sample) in frame.iter().enumerate() {
                        if channels.contains(&chan_idx) {
                            assert_eq!(
                                *sample, chan_idx as f32,
                                "Frame {}, Channel {}",
                                frame_idx, chan_idx
                            );
                        } else {
                            assert_eq!(*sample, 0.0, "Frame {}, Channel {}", frame_idx, chan_idx);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn can_dequeue_audio_into_an_output_buffer_for_multiple_channels() {
        const MAX_NUM_CHANNELS: usize = 10;
        for num_channels in 2..MAX_NUM_CHANNELS {
            for start in 0..num_channels - 1 {
                for end in start + 1..num_channels {
                    let (sender, receiver) = crossbeam::channel::unbounded();

                    let num_frames = 128;
                    let channels = AudioChannelSelection::Range(start..end).as_vec();
                    let process =
                        make_audio_dequeuing_function(receiver, num_channels, channels.clone());

                    let mut buffer_sent = AudioBuffer::with_frames(num_frames, num_channels as u32);
                    assign_channel_index_to_each_sample(&mut buffer_sent);
                    sender.send(buffer_sent.clone()).unwrap();

                    let mut output_buffer = vec![0.0; num_frames as usize * num_channels];
                    process(&mut output_buffer);

                    for (frame_idx, frame) in output_buffer.chunks(num_channels).enumerate() {
                        for (chan_idx, sample) in frame.iter().enumerate() {
                            if channels.contains(&chan_idx) {
                                assert_eq!(
                                    *sample, chan_idx as f32,
                                    "Frame {}, Channel {}",
                                    frame_idx, chan_idx
                                );
                            } else {
                                assert_eq!(
                                    *sample, 0.0,
                                    "Frame {}, Channel {}",
                                    frame_idx, chan_idx
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
