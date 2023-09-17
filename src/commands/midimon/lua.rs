use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use crossbeam::channel::Sender;

pub enum ScriptEvent {
    // TODO: Add Log Event and move Lua VM instance into MIDI thread
    Connect(String),
    Alert(String),
    Pause,
    Resume,
    Stop,
}

mod api {
    use super::*;
    use crossbeam::channel::Sender;

    pub fn add_log(lua: &mlua::Lua, name: &str) -> anyhow::Result<()> {
        let log = lua.create_function({
            let name = name.to_owned();
            move |_, message: String| {
                log::info!("{name} : {}", message);
                Ok(())
            }
        })?;
        lua.globals().set("log", log)?;
        Ok(())
    }

    pub fn add_connect(lua: &mlua::Lua, name: &str, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
        let connect = lua.create_function({
            let name = name.to_owned();
            move |_, device_name: String| {
                if let Err(e) = tx.try_send(ScriptEvent::Connect(device_name)) {
                    log::error!("{name} ! failed to send connection message : {}", e);
                }
                Ok(())
            }
        })?;
        lua.globals().set("connect", connect)?;
        Ok(())
    }

    pub fn add_alert(lua: &mlua::Lua, name: &str, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
        let alert = lua.create_function({
            let name = name.to_owned();
            move |_, device_name: String| {
                if let Err(e) = tx.try_send(ScriptEvent::Alert(device_name)) {
                    log::error!("{name} ! failed to send alert message : {}", e);
                }
                Ok(())
            }
        })?;
        lua.globals().set("alert", alert)?;
        Ok(())
    }

    pub fn add_pause(lua: &mlua::Lua, name: &str, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
        let pause = lua.create_function({
            let name = name.to_owned();
            move |_, (): ()| {
                if let Err(e) = tx.try_send(ScriptEvent::Pause) {
                    log::error!("{name} ! failed to send pause message : {}", e);
                }
                Ok(())
            }
        })?;
        lua.globals().set("pause", pause)?;
        Ok(())
    }

    pub fn add_resume(lua: &mlua::Lua, name: &str, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
        let resume = lua.create_function({
            let name = name.to_owned();
            move |_, (): ()| {
                if let Err(e) = tx.try_send(ScriptEvent::Resume) {
                    log::error!("{name} ! failed to send resume message : {}", e);
                }
                Ok(())
            }
        })?;
        lua.globals().set("resume", resume)?;
        Ok(())
    }

    pub fn add_stop(lua: &mlua::Lua, name: &str, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
        let stop = lua.create_function({
            let name = name.to_owned();
            move |_, (): ()| {
                if let Err(e) = tx.try_send(ScriptEvent::Stop) {
                    log::error!("{name} ! failed to send stop message : {}", e);
                }
                Ok(())
            }
        })?;
        lua.globals().set("stop", stop)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Lua {
    vm: Arc<Mutex<mlua::Lua>>,
    has_loaded_script: Arc<AtomicBool>,
}

unsafe impl Send for Lua {}
unsafe impl Sync for Lua {}

impl Lua {
    pub fn new() -> Self {
        Self {
            vm: Arc::new(Mutex::new(mlua::Lua::new())),
            has_loaded_script: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn load(&self, script: &str) -> anyhow::Result<()> {
        match self.vm.try_lock() {
            Ok(ctx) => ctx.load(script).exec()?,
            Err(_) => anyhow::bail!("Failed to lock Lua VM"),
        }

        self.has_loaded_script.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn has_loaded_script(&self) -> bool {
        self.has_loaded_script.load(Ordering::SeqCst)
    }

    pub fn setup_api(&self, script_filename: &str, tx: Sender<ScriptEvent>) -> anyhow::Result<()> {
        match self.vm.try_lock() {
            Ok(ctx) => {
                api::add_log(&ctx, script_filename)?;
                api::add_connect(&ctx, script_filename, tx.clone())?;
                api::add_alert(&ctx, script_filename, tx.clone())?;
                api::add_pause(&ctx, script_filename, tx.clone())?;
                api::add_resume(&ctx, script_filename, tx.clone())?;
                api::add_stop(&ctx, script_filename, tx.clone())?;
                Ok(())
            }
            Err(_) => anyhow::bail!("Failed to lock Lua VM"),
        }
    }

    fn get(&self, func_name: &'static str) -> anyhow::Result<mlua::OwnedFunction> {
        match self.vm.try_lock() {
            Ok(ctx) => Ok(ctx.globals().get(func_name)?),
            Err(_) => anyhow::bail!("Failed to lock Lua VM"),
        }
    }

    pub fn on_start(&self) -> anyhow::Result<()> {
        Ok(self.get("on_start")?.call(())?)
    }

    pub fn on_discover(&self, device_names: &[String]) -> anyhow::Result<()> {
        Ok(self.get("on_discover")?.call(device_names)?)
    }

    pub fn on_connect(&self, device_name: &str) -> anyhow::Result<()> {
        Ok(self.get("on_connect")?.call(device_name)?)
    }

    pub fn on_midi(&self, device_name: &str, bytes: &[u8]) -> anyhow::Result<Option<bool>> {
        Ok(self.get("on_midi")?.call((device_name, bytes))?)
    }

    pub fn on_tick(&self) -> anyhow::Result<()> {
        Ok(self.get("on_tick")?.call(())?)
    }

    pub fn on_stop(&self) -> anyhow::Result<()> {
        Ok(self.get("on_stop")?.call(())?)
    }
}
