// use ratatui::{prelude::*, widgets::*};
//
// const COLORS: [Color; 8] = [
//     Color::Cyan,
//     Color::Yellow,
//     Color::Magenta,
//     Color::Green,
//     Color::Red,
//     Color::Blue,
//     Color::Gray,
//     Color::LightRed,
// ];
//
// pub fn render<B: Backend>(
//     f: &mut Frame<B>,
//     area: Rect,
//     title: &str,
//     audio: &mut impl AsMut<[f32]>,
//     num_channels: usize,
// ) {
//     const DOWNSAMPLE: usize = 8;
//     let max_samples = f.size().width as usize;
//     let max_x = max_samples as f64;
//
//     let mut data: Vec<Vec<(f64, f64)>> = vec![];
//     for channel in audio.as_mut().chunks_mut(num_channels) {
//         let num_samples_to_drain = (max_samples * DOWNSAMPLE).min(channel.len());
//         let samples: Vec<f32> = channel.drain(0..num_samples_to_drain).collect();
//         let mut channel_data = vec![(0., 0.); max_samples];
//
//         for i in 0..max_samples {
//             channel_data[i].0 = i as f64;
//             channel_data[i].1 = if i * DOWNSAMPLE < samples.len() {
//                 samples[i * DOWNSAMPLE] as f64
//             } else {
//                 0.
//             };
//         }
//
//         channel_data.reverse();
//         data.push(channel_data);
//     }
//
//     let mut datasets: Vec<Dataset> = vec![];
//     for (i, points) in data.iter().enumerate() {
//         datasets.push(
//             Dataset::default()
//                 .name(i.to_string())
//                 .marker(symbols::Marker::Braille)
//                 .style(Style::default().fg(COLORS[i % COLORS.len()]))
//                 .data(points),
//         );
//     }
//
//     let chart = Chart::new(datasets)
//         .block(
//             Block::default()
//                 .title(title.dark_gray())
//                 .borders(Borders::ALL)
//                 .style(Style::default().fg(Color::DarkGray)),
//         )
//         .x_axis(
//             Axis::default()
//                 .style(Style::default().fg(Color::DarkGray))
//                 .bounds([0., max_x]),
//         )
//         .y_axis(
//             Axis::default()
//                 .style(Style::default().fg(Color::DarkGray))
//                 .labels(vec!["-1 ".bold(), "――".into(), " 1 ".bold()])
//                 .bounds([-1.0, 1.0]),
//         );
//
//     f.render_widget(chart, area);
// }
