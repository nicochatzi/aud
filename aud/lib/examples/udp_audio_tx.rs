use aud_lib::audio::*;
use aud_lib::comms::*;
use std::net::UdpSocket;
use std::thread::sleep;
use std::time::Duration;

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
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    setup_logger()?;

    let sockets = Sockets {
        socket: UdpSocket::bind("127.0.0.1:8081").unwrap(),
        target: "127.0.0.1:8080".parse().unwrap(),
    };

    log::info!("socket opened");

    let mut tx = RemoteAudioTransmitter::new(HostAudioInput::default(), sockets).unwrap();

    while !tx.is_accessible() {
        if let Err(e) = tx.process_audio_events() {
            log::error!("failed to process requests : {e}");
        }

        sleep(Duration::from_millis(100));
    }

    log::info!("connected to audio device");

    while tx.is_accessible() {
        tx.process_audio_events().unwrap();
    }

    Ok(())
}
