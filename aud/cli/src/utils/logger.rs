use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Once,
    },
};

static INIT: Once = Once::new();
static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn is_active() -> bool {
    IS_INITIALIZED.load(Ordering::SeqCst)
}

pub fn start(id: &str, file: impl AsRef<Path>, verbose: bool) -> anyhow::Result<()> {
    let level = if verbose {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Debug
    };

    if is_active() {
        anyhow::bail!("attempted to setup logger more than once");
    }

    let id = format!("{}:{}", id.to_owned(), std::process::id());

    fern::Dispatch::new()
        .format(move |out, msg, record| {
            let time = humantime::format_rfc3339_seconds(std::time::SystemTime::now());

            if cfg!(debug_assertions) {
                out.finish(format_args!(
                    "[ {id} ] : [ {time} ] : [ {} {} ] : {msg}",
                    record.target(),
                    record.level(),
                ))
            } else {
                out.finish(format_args!("[ {id} ] : [ {time} ] : {msg}"))
            }
        })
        .level(level)
        .level_for("mio", log::LevelFilter::Off)
        .level_for("notify", log::LevelFilter::Off)
        .chain(fern::log_file(file.as_ref())?)
        .apply()?;

    log::trace!("started");

    INIT.call_once(|| IS_INITIALIZED.store(true, Ordering::SeqCst));
    Ok(())
}
