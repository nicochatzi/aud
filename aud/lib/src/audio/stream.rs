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
            num_channels: config.channels() as usize,
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
        sender: Sender<AudioBuffer>,
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
    sender: Sender<AudioBuffer>,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    selection: AudioChannelSelection,
) -> anyhow::Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let total_num_chanels = config.channels as usize;
    let channels = selection.to_vec();
    let num_requested_channels = channels.len();

    let err_handler = move |e| log::error!("an error occurred in audio stream: {e}",);
    let push_buffer = move |audio_buffer: &[T]| {
        let mut write_chan = 0;
        let num_samples = audio_buffer.len() / total_num_chanels;
        let mut buffer = AudioBuffer::new(num_samples as u32, num_requested_channels as u32);

        for (read_chan, frame) in audio_buffer.chunks(total_num_chanels).enumerate() {
            if !channels.contains(&read_chan) {
                continue;
            }

            for (sample, value) in frame.iter().enumerate() {
                buffer.data[write_chan * buffer.num_channels as usize + sample] =
                    value.to_float_sample().to_sample::<f32>();
            }

            write_chan += 1;
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
