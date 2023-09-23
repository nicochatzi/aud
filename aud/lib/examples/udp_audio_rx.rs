use aud_lib::audio::*;
use aud_lib::comms::*;
use crossbeam::channel::{Receiver, Sender};
use std::net::UdpSocket;

#[derive(Default, Debug)]
pub struct AudioInfo {
    num_samples: u32,
    num_channels: u32,
}

impl From<AudioBuffer> for AudioInfo {
    fn from(buf: AudioBuffer) -> Self {
        Self {
            num_samples: buf.num_frames() as u32,
            num_channels: buf.num_channels,
        }
    }
}

fn main() -> anyhow::Result<()> {
    setup_logger()?;

    let mut rx = RemoteAudioReceiver::with_address(Sockets {
        socket: UdpSocket::bind("127.0.0.1:8080").unwrap(),
        target: "127.0.0.1:8081".parse().unwrap(),
    })
    .unwrap();

    let (sender, receiver) = crossbeam::channel::bounded(100);
    run_buffer_count_logger_task(receiver);
    wait_for_list_of_devices(&mut rx);
    request_audio_device_connection(&mut rx);
    wait_for_audio_device_connection(&mut rx);
    fetch_audio(&mut rx, sender);
    Ok(())
}

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

fn run_buffer_count_logger_task(receiver: Receiver<AudioInfo>) {
    std::thread::spawn(move || loop {
        let stats = receiver
            .try_iter()
            .fold(AudioInfo::default(), |acc, info| AudioInfo {
                num_channels: acc.num_channels + info.num_channels,
                num_samples: acc.num_samples + info.num_samples,
            });

        log::info!("{stats:?}");
        std::thread::sleep(std::time::Duration::from_millis(1_000));
    });
}

fn wait_for_list_of_devices(rx: &mut RemoteAudioReceiver) {
    while rx.list_audio_devices().is_empty() {
        std::thread::sleep(std::time::Duration::from_millis(1_000));
        rx.process_audio_events().unwrap();
        log::info!("reattempting to get devices");
    }
}

fn request_audio_device_connection(rx: &mut RemoteAudioReceiver) {
    let devices = rx.list_audio_devices().to_vec();
    log::info!("found devices : {devices:#?}");
    let channels = AudioChannelSelection::Mono(0);
    rx.connect_to_audio_device(&devices[0], channels.clone())
        .unwrap();
    log::info!(
        "requested connection to : {:#?} with {channels:#?}",
        devices[0].name
    );
}

fn wait_for_audio_device_connection(rx: &mut RemoteAudioReceiver) {
    while rx.retrieve_audio_buffer().data.is_empty() {
        rx.process_audio_events().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn fetch_audio(rx: &mut RemoteAudioReceiver, sender: Sender<AudioInfo>) {
    loop {
        rx.process_audio_events().unwrap();
        let audio = rx.retrieve_audio_buffer();
        if !audio.data.is_empty() {
            sender.send(audio.into()).unwrap();
        }
    }
}
