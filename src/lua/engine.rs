use super::LuaRuntime;
use crossbeam::channel::Receiver;

pub trait LuaRuntimeControlling: Clone + std::marker::Send {
    fn run(&mut self, runtime: &mut LuaRuntime) -> anyhow::Result<()>;
}

pub struct LuaEngineHandle {
    handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>,
    rx: Receiver<LuaEngineEvent>,
}

impl LuaEngineHandle {
    pub fn engine_events(&mut self) -> Receiver<LuaEngineEvent> {
        self.rx.clone()
    }

    pub fn take_handle(&mut self) -> Option<std::thread::JoinHandle<anyhow::Result<()>>> {
        self.handle.take()
    }
}

pub enum LuaEngineEvent {
    Panicked,
    Terminated,
}

pub fn start_engine<C>(controller: C) -> LuaEngineHandle
where
    C: LuaRuntimeControlling + 'static,
{
    let (tx, rx) = crossbeam::channel::unbounded();

    let handle = std::thread::spawn({
        move || {
            let controller = controller.clone();

            loop {
                let runtime_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe({
                    let controller = controller.clone();
                    move || {
                        let mut runtime = crate::lua::LuaRuntime::default();
                        let mut controller = controller.clone();
                        controller.run(&mut runtime).unwrap();
                    }
                }));

                match runtime_result {
                    Ok(_) => {
                        if let Err(e) = tx.try_send(LuaEngineEvent::Terminated) {
                            log::error!("Failed to send Lua Engine event : {e}");
                        }

                        log::trace!("Lua Runtime terminated");
                        return Ok(());
                    }
                    Err(err) => {
                        if let Err(e) = tx.try_send(LuaEngineEvent::Panicked) {
                            log::error!("Failed to send Lua Engine event : {e}");
                        }

                        if let Some(string_message) = err.downcast_ref::<&str>() {
                            log::error!("Lua Runtime Panic : {}", string_message);
                        } else if let Some(string_message) = err.downcast_ref::<String>() {
                            log::error!("Lua Runtime Panic : {}", string_message);
                        } else {
                            log::error!("Lua Runtime Panic : {:?}", err);
                        }
                    }
                }
            }
        }
    });

    LuaEngineHandle {
        handle: Some(handle),
        rx,
    }
}
