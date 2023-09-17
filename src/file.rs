use crossbeam::channel::Receiver;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;

pub fn watch(
    path: impl AsRef<Path>,
) -> anyhow::Result<Receiver<Result<notify::Event, notify::Error>>> {
    let (tx, rx) = crossbeam::channel::bounded(100);
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;
    Ok(rx)
}

/// Default locations stored in `~/.aud`
///
/// .
/// ├── api
/// │  ├── aud/
/// │  ├── examples/
/// │  └── midimon/
/// ├── bin
/// │  └── aud
/// └── log
///    └── aud.log
///
pub mod locations {
    use std::path::PathBuf;

    pub fn aud() -> Option<PathBuf> {
        Some(dirs::home_dir()?.join(".aud"))
    }

    pub fn bin() -> Option<PathBuf> {
        Some(aud()?.join("bin"))
    }

    pub fn api() -> Option<PathBuf> {
        Some(aud()?.join("api"))
    }

    pub fn log() -> Option<PathBuf> {
        Some(aud()?.join("log"))
    }
}
