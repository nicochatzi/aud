pub mod audio;
pub mod comms;
pub mod controllers;
pub mod dsp;
pub mod files;
pub mod lua;
pub mod midi;

#[cfg(test)]
pub(crate) mod test {
    use std::path::{Path, PathBuf};

    pub fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("AUD_LIB_FIXTURES"))
            .canonicalize()
            .unwrap()
    }

    pub fn fixture(name: impl AsRef<Path>) -> PathBuf {
        fixtures_dir().join(name)
    }
}
