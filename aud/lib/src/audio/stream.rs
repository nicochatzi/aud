use super::*;
use cpal::{traits::*, FromSample, Sample, SizedSample};
use crossbeam::channel::Sender;

pub enum Direction {
    Input,
    Output,
}

pub fn build_audio_device_list(host: &cpal::Host) -> Vec<AudioDevice> {
    match host.input_devices() {
        Ok(devices) => devices.filter_map(AudioDevice::try_from_input).collect(),
        Err(err) => {
            log::error!("Failed to get input devices: {}", err);
            vec![]
        }
    }
}

#[derive(Default)]
pub struct AudioStream {
    stream: Option<cpal::Stream>,
}

impl AudioDevice {
    pub fn try_from_output(device: cpal::Device) -> Option<Self> {
        Some(Self::try_from_config(
            device.name().ok()?,
            device.default_output_config().ok()?,
        ))
    }

    pub fn try_from_input(device: cpal::Device) -> Option<Self> {
        Some(Self::try_from_config(
            device.name().ok()?,
            device.default_input_config().ok()?,
        ))
    }

    fn try_from_config(name: String, config: cpal::SupportedStreamConfig) -> Self {
        Self {
            name,
            channels: config.channels() as usize,
        }
    }
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
        }
    }
}

impl AudioStream {
    pub fn is_open(&self) -> bool {
        self.stream.is_some()
    }

    pub fn open(
        sender: Sender<Vec<Vec<f32>>>,
        dev: &cpal::Device,
        sel: AudioChannelSelection,
    ) -> anyhow::Result<Self> {
        let config = dev.default_input_config()?;
        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => run::<i8>(sender, dev, &config.into(), sel),
            cpal::SampleFormat::I16 => run::<i16>(sender, dev, &config.into(), sel),
            cpal::SampleFormat::I32 => run::<i32>(sender, dev, &config.into(), sel),
            cpal::SampleFormat::I64 => run::<i64>(sender, dev, &config.into(), sel),
            cpal::SampleFormat::U8 => run::<u8>(sender, dev, &config.into(), sel),
            cpal::SampleFormat::U16 => run::<u16>(sender, dev, &config.into(), sel),
            cpal::SampleFormat::U32 => run::<u32>(sender, dev, &config.into(), sel),
            cpal::SampleFormat::U64 => run::<u64>(sender, dev, &config.into(), sel),
            cpal::SampleFormat::F32 => run::<f32>(sender, dev, &config.into(), sel),
            cpal::SampleFormat::F64 => run::<f64>(sender, dev, &config.into(), sel),
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
            log::error!("failed to push audio out of CPAL: {e}");
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
