use cpal::{traits::*, FromSample, Sample, SizedSample};
use crossbeam::channel::Sender;

#[derive(Copy, Clone)]
pub enum Direction {
    In,
    Out,
}

impl Direction {
    fn as_str(self) -> &'static str {
        match self {
            Direction::In => "input",
            Direction::Out => "output",
        }
    }
}

pub fn open(
    sender: Sender<Vec<Vec<f32>>>,
    device: &cpal::Device,
    config: cpal::SupportedStreamConfig,
    direction: Direction,
) -> anyhow::Result<cpal::Stream> {
    match config.sample_format() {
        cpal::SampleFormat::I8 => run::<i8>(sender, device, &config.into(), direction),
        cpal::SampleFormat::I16 => run::<i16>(sender, device, &config.into(), direction),
        cpal::SampleFormat::I32 => run::<i32>(sender, device, &config.into(), direction),
        cpal::SampleFormat::I64 => run::<i64>(sender, device, &config.into(), direction),
        cpal::SampleFormat::U8 => run::<u8>(sender, device, &config.into(), direction),
        cpal::SampleFormat::U16 => run::<u16>(sender, device, &config.into(), direction),
        cpal::SampleFormat::U32 => run::<u32>(sender, device, &config.into(), direction),
        cpal::SampleFormat::U64 => run::<u64>(sender, device, &config.into(), direction),
        cpal::SampleFormat::F32 => run::<f32>(sender, device, &config.into(), direction),
        cpal::SampleFormat::F64 => run::<f64>(sender, device, &config.into(), direction),
        sample_format => anyhow::bail!("Unsupported sample format '{sample_format}'"),
    }
}

fn run<T>(
    sender: Sender<Vec<Vec<f32>>>,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    direction: Direction,
) -> anyhow::Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;

    let err_handler = move |e| {
        log::error!(
            "an error occurred in audio {} stream: {e}",
            direction.as_str()
        )
    };

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

    let stream = match direction {
        Direction::In => device.build_input_stream(
            config,
            move |data: &[T], _| push_buffer(data),
            err_handler,
            None,
        )?,
        Direction::Out => device.build_output_stream(
            config,
            move |data: &mut [T], _| push_buffer(data),
            err_handler,
            None,
        )?,
    };

    stream.play()?;

    log::trace!("started audio {} stream", direction.as_str());

    Ok(stream)
}
