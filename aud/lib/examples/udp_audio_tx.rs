use aud_lib::audio::*;
use aud_lib::comms::*;
use std::net::UdpSocket;

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

    let mut tx = AudioTransmitter::new(sockets, HostedAudioProducer::default()).unwrap();

    while !tx.is_audio_connected() {
        if let Err(e) = tx.process_requests() {
            log::error!("failed to process requests : {e}");
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    log::info!("connected to audio device");

    loop {
        if let Err(e) = tx.try_send_audio() {
            log::error!("failed to send audio : {e}");
        }
    }

    Ok(())
}
