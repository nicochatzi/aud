use aud_cli::{cmd::*, logger, terminal::with_terminal};
use clap::{CommandFactory, Parser, Subcommand};
use std::io::Write;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// MIDI input monitor
    Midimon(midimon::Options),
    /// Ableton Link controller
    Derlink(derlink::Options),
    /// Audio oscilloscope
    Auscope(auscope::Options),
    /// `aud completions --generate=zsh > aud.zsh`
    Completions(Completions),
}

#[derive(Debug, Parser)]
#[command(arg_required_else_help(true))]
struct Completions {
    /// shell to generate the completion script for
    #[arg(long = "generate", value_enum)]
    shell: Option<clap_complete::Shell>,
}

impl Completions {
    fn generate(&self) -> anyhow::Result<()> {
        let Some(shell) = self.shell else {
            anyhow::bail!("no shell specified for autocompletion generation");
        };

        let mut stdout = std::io::stdout();
        stdout.flush()?;

        let mut cli = Cli::command();
        clap_complete::generate(shell, &mut cli, "aud", &mut stdout);

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    if let Commands::Completions(ref c) = args.command {
        return c.generate();
    }

    let app_result = with_terminal(move |term| match args.command {
        Commands::Midimon(opts) => midimon::run(term, opts),
        Commands::Derlink(opts) => derlink::run(term, opts),
        Commands::Auscope(opts) => auscope::run(term, opts),
        Commands::Completions(_) => Ok(()),
    });

    if let Err(e) = app_result {
        if logger::is_active() {
            log::error!("{e}");
        } else {
            use colored::*;
            eprintln!("{} {}", "Error:".red().bold(), format!("{e}").bold());
        }
    }

    Ok(())
}
