use clap::{CommandFactory, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// MIDI input monitor
    Midimon(aud::commands::midimon::Options),
    /// Ableton Link controller
    Derlink(aud::commands::derlink::Options),
    /// Audio oscilloscope
    Auscope(aud::commands::auscope::Options),
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

        use std::io::Write;
        std::io::stdout().flush()?;

        let mut cli = Cli::command();
        clap_complete::generate(shell, &mut cli, "aud", &mut std::io::stdout());

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    if let Commands::Completions(ref c) = args.command {
        return c.generate();
    }

    let app_result = aud::terminal::with_terminal(move |term| match args.command {
        Commands::Midimon(opts) => aud::commands::midimon::run(term, opts),
        Commands::Derlink(opts) => aud::commands::derlink::run(term, opts),
        Commands::Auscope(opts) => aud::commands::auscope::run(term, opts),
        _ => Ok(()),
    });

    if let Err(e) = app_result {
        if aud::logger::is_active() {
            log::error!("{e}");
        } else {
            use colored::*;
            eprintln!("{} {}", "Error:".red().bold(), format!("{e}").bold());
        }
    }

    Ok(())
}
