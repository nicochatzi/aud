use aud_lib::audio::*;
use criterion::{criterion_group, criterion_main, Criterion};
use rand::random;
use std::iter::repeat_with;

fn bench_enqueue(c: &mut Criterion) {
    for buffer_size in [128, 256, 512, 1024] {
        let mut group = c.benchmark_group(format!("Enqueue Mono Buffer Size {}", buffer_size));
        group.bench_function("Mono", |b| {
            let (sender, _receiver) = crossbeam::channel::unbounded();
            let input_channels = 1;
            let selected_channels = vec![0];
            let enqueue_fn = test_make_audio_buffer_enqueueing_function(
                sender,
                input_channels,
                selected_channels,
            );
            let audio_buffer: Vec<f32> = repeat_with(|| random::<f32>())
                .take(buffer_size * input_channels)
                .collect();
            b.iter(|| {
                enqueue_fn(&audio_buffer);
            });
        });
        group.finish();
    }
}

fn bench_dequeue(c: &mut Criterion) {
    for buffer_size in [128, 256, 512, 1024] {
        let mut group = c.benchmark_group(format!("Dequeue Stereo Buffer Size {}", buffer_size));
        group.bench_function("Stereo", |b| {
            let (sender, receiver) = crossbeam::channel::unbounded();
            let total_channels = 2;
            let selected_channels = vec![0, 1];
            let dequeue_fn = test_make_audio_dequeing_function(
                receiver.clone(),
                total_channels,
                selected_channels,
            );
            std::vec::from_elem(buffer_size * total_channels, 0);
            let mut audio_buffer: Vec<f32> = vec![0.0; buffer_size * total_channels];

            b.iter(|| {
                let mock_data = AudioBuffer::with_length(
                    (buffer_size * total_channels) as u32,
                    total_channels as u32,
                );
                sender.send(mock_data.clone()).unwrap(); // Clone the mock data
                dequeue_fn(&mut audio_buffer);
            });
        });
        group.finish();
    }
}

criterion_group!(host_audio_io, bench_enqueue, bench_dequeue);
criterion_main!(host_audio_io);
