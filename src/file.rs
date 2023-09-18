use crossbeam::channel::Receiver;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};

/// Watch a path
///
/// Returns a `crossbeam::channel::Receiver<notify::Event>`
/// that the caller can use to poll for events on that path.
pub fn watch(path: impl AsRef<Path>) -> anyhow::Result<Receiver<notify::Result<notify::Event>>> {
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
    use super::*;

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

pub mod files {
    use super::*;

    pub fn log() -> Option<PathBuf> {
        Some(locations::log()?.join("aud.log"))
    }
}
