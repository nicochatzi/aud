use super::*;
use cpal::{traits::*, FromSample, Sample, SizedSample};
use crossbeam::channel::{Receiver, Sender};

pub struct HostedAudioProducer {
    host: cpal::Host,
    sender: Sender<AudioBuffer>,
    receiver: Receiver<AudioBuffer>,
    stream: AudioStream,
    devices: Vec<AudioDevice>,
}

impl Default for HostedAudioProducer {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(100);
        let host = cpal::default_host();

        Self {
            stream: AudioStream::default(),
            sender,
            receiver,
            devices: build_audio_device_list(&host),
            host,
        }
    }
}

impl AudioProviding for HostedAudioProducer {
    fn connect_to_audio_device(
        &mut self,
        audio_device: &AudioDevice,
        channel_selection: AudioChannelSelection,
    ) -> anyhow::Result<()> {
        if channel_selection.is_valid_for_device(audio_device) {
            log::error!("Invalid selection : {channel_selection:#?} for : {audio_device:#?}");
            return Ok(());
        }

        self.stream = self
            .host
            .input_devices()?
            .find(|device| device.name().ok().as_deref() == Some(&audio_device.name))
            .map(|device| AudioStream::open(self.sender.clone(), &device, channel_selection))
            .ok_or_else(|| anyhow::anyhow!("No audio device selected"))??;

        Ok(())
    }

    fn list_audio_devices(&self) -> &[AudioDevice] {
        self.devices.as_slice()
    }

    fn try_fetch_audio(&mut self) -> anyhow::Result<AudioBuffer> {
        Ok(self.receiver.try_recv()?)
    }
}

pub fn build_audio_device_list(host: &cpal::Host) -> Vec<AudioDevice> {
    match host.input_devices() {
        Ok(devices) => devices.filter_map(AudioDevice::try_from).collect(),
        Err(err) => {
            log::error!("Failed to get input devices: {}", err);
            vec![]
        }
    }
}

impl AudioDevice {
    fn try_from(device: cpal::Device) -> Option<Self> {
        let name = device.name().ok()?;
        device.default_input_config().ok().map(|config| Self {
            name,
            channels: config.channels() as usize,
        })
    }
}

#[derive(Default)]
struct AudioStream {
    stream: Option<cpal::Stream>,
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
        }
    }
}

impl AudioStream {
    fn open(
        sender: Sender<Vec<Vec<f32>>>,
        device: &cpal::Device,
        selection: AudioChannelSelection,
    ) -> anyhow::Result<Self> {
        let config = device.default_input_config()?;
        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => run::<i8>(sender, device, &config.into(), selection),
            cpal::SampleFormat::I16 => run::<i16>(sender, device, &config.into(), selection),
            cpal::SampleFormat::I32 => run::<i32>(sender, device, &config.into(), selection),
            cpal::SampleFormat::I64 => run::<i64>(sender, device, &config.into(), selection),
            cpal::SampleFormat::U8 => run::<u8>(sender, device, &config.into(), selection),
            cpal::SampleFormat::U16 => run::<u16>(sender, device, &config.into(), selection),
            cpal::SampleFormat::U32 => run::<u32>(sender, device, &config.into(), selection),
            cpal::SampleFormat::U64 => run::<u64>(sender, device, &config.into(), selection),
            cpal::SampleFormat::F32 => run::<f32>(sender, device, &config.into(), selection),
            cpal::SampleFormat::F64 => run::<f64>(sender, device, &config.into(), selection),
            sample_format => anyhow::bail!("Unsupported sample format '{sample_format}'"),
        }?;

        Ok(Self {
            stream: Some(stream),
        })
    }
}

fn run<T>(
    sender: Sender<Vec<Vec<f32>>>,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    selection: AudioChannelSelection,
) -> anyhow::Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let num_channels = config.channels as usize;
    let channels = selection.to_vec();

    let err_handler = move |e| log::error!("an error occurred in audio stream: {e}",);
    let push_buffer = move |data: &[T]| {
        let mut buffer = vec![vec![0.; data.len() / num_channels]; num_channels];

        for (chan, frame) in data.chunks(num_channels).enumerate() {
            if !channels.contains(&chan) {
                continue;
            }

            for (sample, value) in frame.iter().enumerate() {
                buffer[sample][chan] = value.to_float_sample().to_sample::<f32>();
            }
        }

        if let Err(e) = sender.try_send(buffer) {
            log::error!("failed to push audio: {e}");
        }
    };

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| push_buffer(data),
        err_handler,
        None,
    )?;

    stream.play()?;

    log::trace!("started audio stream");

    Ok(stream)
}
