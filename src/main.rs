use aud::commands::*;
use clap::{CommandFactory, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Disable the terminal UI, headless mode
    #[arg(long)]
    headless: bool,

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

fn run_with_tui(args: Cli) -> anyhow::Result<()> {
    aud::terminal::with_terminal(move |term| match args.command {
        Commands::Midimon(opts) => midimon::run_with_tui(term, opts),
        Commands::Derlink(opts) => derlink::run_with_tui(term, opts),
        Commands::Auscope(opts) => auscope::run_with_tui(term, opts),
        Commands::Completions(_) => Ok(()),
    })
}

fn run_headless(args: Cli) -> anyhow::Result<()> {
    match args.command {
        Commands::Midimon(opts) => midimon::run_headless(opts),
        Commands::Derlink(opts) => derlink::run_headless(opts),
        Commands::Auscope(opts) => auscope::run_headless(opts),
        Commands::Completions(_) => Ok(()),
    }
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    if let Commands::Completions(ref c) = args.command {
        return c.generate();
    }

    let app_result = if args.headless {
        run_headless(args)
    } else {
        run_with_tui(args)
    };

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
