use aud_lib::audio::*;
use aud_lib::comms::*;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::sleep;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Default, Debug)]
pub struct AudioInfo {
    num_samples: u32,
    num_buffers: u32,
    num_silence: usize,
}

struct LoggingAudioConsumer {
    buffers: Arc<Mutex<Vec<AudioBuffer>>>,
    _handle: JoinHandle<()>,
}

impl Default for LoggingAudioConsumer {
    fn default() -> Self {
        let buffers = Arc::new(Mutex::new(vec![]));
        let handle = std::thread::spawn({
            let buffers = buffers.clone();

            move || loop {
                let stats = buffers.try_lock().unwrap().iter().fold(
                    AudioInfo::default(),
                    |stats: AudioInfo, buffer: &AudioBuffer| AudioInfo {
                        num_samples: stats.num_samples + buffer.num_frames() as u32,
                        num_buffers: stats.num_buffers + 1,
                        num_silence: stats.num_silence
                            + buffer
                                .data
                                .iter()
                                .all(|s| *s == 0.)
                                .then_some(1)
                                .unwrap_or_default(),
                    },
                );
                buffers.try_lock().unwrap().clear();
                log::info!(
                    "last second : received {} samples, with {} buffers, with {} silence buffers",
                    stats.num_samples,
                    stats.num_buffers,
                    stats.num_silence,
                );
                sleep(Duration::from_millis(1_000));
            }
        });

        Self {
            _handle: handle,
            buffers,
        }
    }
}

impl AudioConsuming for LoggingAudioConsumer {
    fn consume_audio_buffer(&mut self, buffer: AudioBuffer) -> anyhow::Result<()> {
        let mut buffers = self.buffers.lock().unwrap();
        buffers.push(buffer);
        Ok(())
    }
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

fn request_audio_device_connection(rx: &mut RemoteAudioReceiver<LoggingAudioConsumer>) {
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

fn main() -> anyhow::Result<()> {
    setup_logger()?;

    let mut rx = RemoteAudioReceiver::new(
        LoggingAudioConsumer::default(),
        Sockets {
            socket: UdpSocket::bind("127.0.0.1:8080").unwrap(),
            target: "127.0.0.1:8081".parse().unwrap(),
        },
    )
    .unwrap();

    while rx.list_audio_devices().is_empty() {
        sleep(Duration::from_millis(1_000));
        rx.process_audio_events().unwrap();
        log::info!("reattempting to get devices");
    }

    request_audio_device_connection(&mut rx);

    while !rx.is_accessible() {
        rx.process_audio_events().unwrap();
        sleep(Duration::from_millis(100));
    }

    while rx.is_accessible() {
        rx.process_audio_events().unwrap();
        sleep(Duration::from_millis(10));
    }

    Ok(())
}
