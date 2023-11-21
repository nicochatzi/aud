use aud::{audio::AudioBuffer, dsp};
use ratatui::{prelude::*, widgets::*};

const COLORS: [Color; 8] = [
    Color::Cyan,
    Color::Yellow,
    Color::Magenta,
    Color::Green,
    Color::Red,
    Color::Blue,
    Color::Gray,
    Color::LightRed,
];

type SamplePoint = (f64, f64);
type SamplePoints = Vec<SamplePoint>;

fn prepare_audio_data(
    audio: &AudioBuffer,
    downsample: usize,
    num_samples_to_render: usize,
) -> Vec<SamplePoints> {
    let num_channels = audio.num_channels.min(1) as usize;
    let audio = dsp::deinterleave(&audio.data, num_channels);
    let mut channels = Vec::<SamplePoints>::with_capacity(num_channels);
    for chan in audio {
        let data = chan
            .iter()
            .take(num_samples_to_render * downsample)
            .rev()
            .step_by(num_channels)
            .step_by(downsample)
            .enumerate()
            .map(|(i, &sample)| (i as f64, sample as f64))
            .collect();
        channels.push(data);
    }
    channels
}

fn create_datasets(data: &[SamplePoints]) -> Vec<Dataset> {
    data.iter()
        .enumerate()
        .map(|(i, points)| {
            Dataset::default()
                .name(i.to_string())
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(COLORS[i % COLORS.len()]))
                .data(points)
        })
        .collect()
}

pub fn render(f: &mut Frame, area: Rect, title: &str, audio: &AudioBuffer, downsample: usize) {
    let width = f.size().width as usize;
    let num_samples_to_render = (audio.num_frames() / downsample).min(width);
    let data = prepare_audio_data(audio, downsample, num_samples_to_render);

    let datasets = create_datasets(&data);

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(title.dark_gray())
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::DarkGray)),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0., f.size().width as f64]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .labels(vec!["-1 ".bold(), "――".into(), " 1 ".bold()])
                .bounds([-1.0, 1.0]),
        );

    f.render_widget(chart, area);
}
