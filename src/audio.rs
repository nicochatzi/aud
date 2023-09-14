use cpal::{traits::*, FromSample, Sample, SizedSample};
use crossbeam::channel::Sender;

pub enum Dir {
    In,
    Out,
}

pub fn stream(
    sender: Sender<Vec<Vec<f32>>>,
    device: &cpal::Device,
    config: cpal::SupportedStreamConfig,
    dir: Dir,
) -> anyhow::Result<cpal::Stream> {
    match config.sample_format() {
        cpal::SampleFormat::I8 => run::<i8>(sender, device, &config.into(), dir),
        cpal::SampleFormat::I16 => run::<i16>(sender, device, &config.into(), dir),
        cpal::SampleFormat::I32 => run::<i32>(sender, device, &config.into(), dir),
        cpal::SampleFormat::I64 => run::<i64>(sender, device, &config.into(), dir),
        cpal::SampleFormat::U8 => run::<u8>(sender, device, &config.into(), dir),
        cpal::SampleFormat::U16 => run::<u16>(sender, device, &config.into(), dir),
        cpal::SampleFormat::U32 => run::<u32>(sender, device, &config.into(), dir),
        cpal::SampleFormat::U64 => run::<u64>(sender, device, &config.into(), dir),
        cpal::SampleFormat::F32 => run::<f32>(sender, device, &config.into(), dir),
        cpal::SampleFormat::F64 => run::<f64>(sender, device, &config.into(), dir),
        sample_format => anyhow::bail!("Unsupported sample format '{sample_format}'"),
    }
}

fn run<T>(
    sender: Sender<Vec<Vec<f32>>>,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    dir: Dir,
) -> anyhow::Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;

    let err_handler = |e| log::error!("an error occurred on stream: {e}");
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

    let stream = match dir {
        Dir::In => device.build_input_stream(
            config,
            move |data: &[T], _| push_buffer(data),
            err_handler,
            None,
        )?,
        Dir::Out => device.build_output_stream(
            config,
            move |data: &mut [T], _| push_buffer(data),
            err_handler,
            None,
        )?,
    };

    stream.play()?;

    Ok(stream)
}
