use super::*;
use cpal::{traits::*, FromSample, Sample, SizedSample};
use crossbeam::channel::{Receiver, Sender};

pub struct HostedAudioReceiver {
    host: cpal::Host,
    sender: Sender<AudioBuffer>,
    receiver: Receiver<AudioBuffer>,
    stream: Option<cpal::Stream>,
    device: Option<cpal::Device>,
}

impl Default for HostedAudioReceiver {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(100);
        let host = cpal::default_host();

        Self {
            host,
            stream: None,
            device: None,
            sender,
            receiver,
        }
    }
}

impl AudioReceiving for HostedAudioReceiver {
    fn list_devices(&self) -> anyhow::Result<Vec<String>> {
        Ok(self
            .host
            .input_devices()?
            .filter_map(|x| x.name().ok())
            .collect())
    }

    fn select_device(&mut self, device_name: &str) -> anyhow::Result<()> {
        self.device = self
            .host
            .input_devices()?
            .find(|x| x.name().map(|y| y == device_name).unwrap_or(false));
        Ok(())
    }

    fn open_stream(&mut self) -> anyhow::Result<()> {
        if let Some(stream) = self.stream.take() {
            stream.pause()?;
        }

        let stream = {
            match self.device {
                Some(ref device) => open(self.sender.clone(), device)?,
                None => anyhow::bail!("No audio device selected"),
            }
        };
        self.stream = Some(stream);
        Ok(())
    }

    fn try_receive_audio(&mut self) -> anyhow::Result<AudioBuffer> {
        Ok(self.receiver.try_recv()?)
    }
}

pub fn open(sender: Sender<Vec<Vec<f32>>>, device: &cpal::Device) -> anyhow::Result<cpal::Stream> {
    let config = device.default_input_config()?;

    match config.sample_format() {
        cpal::SampleFormat::I8 => run::<i8>(sender, device, &config.into()),
        cpal::SampleFormat::I16 => run::<i16>(sender, device, &config.into()),
        cpal::SampleFormat::I32 => run::<i32>(sender, device, &config.into()),
        cpal::SampleFormat::I64 => run::<i64>(sender, device, &config.into()),
        cpal::SampleFormat::U8 => run::<u8>(sender, device, &config.into()),
        cpal::SampleFormat::U16 => run::<u16>(sender, device, &config.into()),
        cpal::SampleFormat::U32 => run::<u32>(sender, device, &config.into()),
        cpal::SampleFormat::U64 => run::<u64>(sender, device, &config.into()),
        cpal::SampleFormat::F32 => run::<f32>(sender, device, &config.into()),
        cpal::SampleFormat::F64 => run::<f64>(sender, device, &config.into()),
        sample_format => anyhow::bail!("Unsupported sample format '{sample_format}'"),
    }
}

fn run<T>(
    sender: Sender<Vec<Vec<f32>>>,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
) -> anyhow::Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;

    let err_handler = move |e| log::error!("an error occurred in audio stream: {e}",);
    let push_buffer = move |data: &[T]| {
        let mut buffer = vec![vec![0.; data.len() / channels]; channels];

        for (chan, frame) in data.chunks(channels).enumerate() {
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
