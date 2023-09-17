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
