use crossbeam::channel::Receiver;
use notify::Watcher;
use std::path::Path;

pub struct FsWatcher {
    _watcher: notify::RecommendedWatcher,
    events: Receiver<notify::Result<notify::Event>>,
}

impl FsWatcher {
    pub fn run(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let (tx, rx) = crossbeam::channel::bounded(100);
        let mut watcher = notify::RecommendedWatcher::new(tx, notify::Config::default())?;
        watcher.watch(path.as_ref(), notify::RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            events: rx,
        })
    }

    pub fn events(&self) -> Receiver<notify::Result<notify::Event>> {
        self.events.clone()
    }
}

pub fn list_with_extension(dir: impl AsRef<Path>, ext: &str) -> anyhow::Result<Vec<String>> {
    let filenames: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some(ext) {
                return path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string());
            }
            None
        })
        .collect();

    Ok(filenames)
}
