use aud_lib::streams::audio::*;
use aud_lib::streams::net::*;
use crossbeam::channel::{Receiver, Sender};
use std::net::{SocketAddr, UdpSocket};
use std::time::SystemTime;

#[derive(Default, Debug)]
struct AudioInfo {
    num_channels: usize,
    num_samples: usize,
}

impl From<AudioBuffer> for AudioInfo {
    fn from(buffer: AudioBuffer) -> Self {
        Self {
            num_samples: buffer.get(0).and_then(|chan| Some(chan.len())).unwrap_or(0),
            num_channels: buffer.len(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    setup_logger()?;

    let mut rx = AudioReceiver::with_address(Sockets {
        socket: UdpSocket::bind("127.0.0.1:8080").unwrap(),
        target: "127.0.0.1:8081".parse().unwrap(),
    })
    .unwrap();

    let (sender, receiver) = crossbeam::channel::bounded::<AudioInfo>(100);
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
            .fold(AudioInfo::default(), |stats, info| AudioInfo {
                num_channels: stats.num_channels + info.num_channels,
                num_samples: stats.num_samples + info.num_samples,
            });
        log::info!("{stats:#?}");
        std::thread::sleep(std::time::Duration::from_millis(1_000));
    });
}

fn wait_for_list_of_devices(rx: &mut AudioReceiver) {
    while rx.list_audio_devices().is_empty() {
        let _ = rx.try_fetch_audio().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1_000));
        log::info!("reattempting to get devices");
    }
}

fn request_audio_device_connection(rx: &mut AudioReceiver) {
    let devices = rx.list_audio_devices().to_vec();
    log::info!("found devices : {devices:#?}");
    rx.connect_to_audio_device(&devices[0], AudioChannelSelection::Mono(0))
        .unwrap();
    log::info!("requested connection to : {:#?}", devices[0].name);
}

fn wait_for_audio_device_connection(rx: &mut AudioReceiver) {
    while rx.try_fetch_audio().unwrap().is_empty() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn fetch_audio(rx: &mut AudioReceiver, sender: Sender<AudioInfo>) {
    loop {
        let audio = rx.try_fetch_audio().unwrap();
        if !audio.is_empty() {
            sender.send(audio.into()).unwrap();
        }
    }
}
