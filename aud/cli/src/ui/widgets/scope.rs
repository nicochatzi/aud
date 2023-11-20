use aud::audio::AudioBuffer;
use ratatui::{prelude::*, widgets::*};

const DOWNSAMPLE: usize = 8;
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

type ChannelData = Vec<(f64, f64)>;

fn prepare_audio_data(audio: &AudioBuffer) -> Vec<ChannelData> {
    audio
        .data
        .chunks(audio.num_channels.max(1) as usize)
        .map(|channel: &[f32]| -> ChannelData {
            channel
                .iter()
                .step_by(DOWNSAMPLE)
                .enumerate()
                .map(|(i, &sample)| (i as f64, sample as f64))
                .collect()
        })
        .collect()
}

fn create_datasets(data: &[ChannelData]) -> Vec<Dataset> {
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

pub fn render(f: &mut Frame, area: Rect, title: &str, audio: &AudioBuffer) -> usize {
    let data = prepare_audio_data(audio);
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
    data.iter().map(Vec::len).sum()
}
