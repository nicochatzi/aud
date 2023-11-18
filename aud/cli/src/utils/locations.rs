/// Default locations stored in `~/.aud`
///
/// .
/// ├── bin
/// │  └── aud
/// ├── log
/// │  └── aud.log
/// └── lua
///    ├── api
///    │  ├── auscope
///    │  ├── midimon
///    │  └── sysexio
///    ├── aud/
///    └── examples
///       ├── auscope
///       ├── midimon
///       └── sysexio
///
use std::path::PathBuf;

pub fn aud() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".aud"))
}

pub fn bin() -> Option<PathBuf> {
    Some(aud()?.join("bin"))
}

pub fn lua() -> Option<PathBuf> {
    Some(aud()?.join("lua"))
}

pub fn log() -> Option<PathBuf> {
    Some(aud()?.join("log"))
}

pub fn log_file(name: &str) -> Option<std::path::PathBuf> {
    Some(log()?.join(format!("{name}.log")))
}

pub mod lua {
    use super::*;

    pub fn lib() -> Option<PathBuf> {
        Some(lua()?.join("examples"))
    }

    pub fn api() -> Option<PathBuf> {
        Some(lua()?.join("examples"))
    }

    pub fn examples() -> Option<PathBuf> {
        Some(lua()?.join("examples"))
    }

    pub fn examples_for(cmd: impl AsRef<str>) -> Option<PathBuf> {
        Some(examples()?.join(cmd.as_ref()))
    }
}
