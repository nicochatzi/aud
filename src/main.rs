use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// MIDI input monitor
    Midimon,
    /// Ableton Link controller
    Derlink,
    /// Audio oscilloscope
    Auscope,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let mut terminal = aud::terminal::acquire()?;
    aud::terminal::set_panic_hook();

    let app_result = match args.command {
        Commands::Midimon => aud::commands::midimon::run(&mut terminal),
        Commands::Derlink => aud::commands::derlink::run(&mut terminal),
        Commands::Auscope => aud::commands::auscope::run(&mut terminal),
    };

    aud::terminal::release()?;

    if let Err(e) = app_result {
        eprintln!("{e}");
    }

    Ok(())
}
