use std::path::Path;

/// `LuaRuntime` is the type that
/// actually runs a script. It
/// needs to be run in a single
/// thread.
pub struct LuaRuntime {
    ctx: mlua::Lua,
    script: Option<String>,
}

impl Default for LuaRuntime {
    fn default() -> Self {
        Self {
            ctx: mlua::Lua::new(),
            script: None,
        }
    }
}

impl LuaRuntime {
    pub fn release_script(&mut self) -> Option<String> {
        self.script.take()
    }

    pub fn has_script(&self) -> bool {
        self.script.is_some()
    }

    pub fn load_file(&mut self, script: impl AsRef<Path>) -> anyhow::Result<()> {
        let script = script.as_ref();
        let is_lua_file = script.exists() && script.is_file() && script.ends_with(".lua");
        if !is_lua_file {
            anyhow::bail!("Invalid Lua script path : {}", script.display());
        }

        self.load_chunk(&std::fs::read_to_string(script)?)
    }

    pub fn load_chunk(&mut self, chunk: &str) -> anyhow::Result<()> {
        self.ctx.load(chunk).exec()?;
        self.script = Some(chunk.into());
        Ok(())
    }

    pub fn call<'lua, A, R>(&'lua self, func_name: &str, args: A) -> anyhow::Result<R>
    where
        A: mlua::IntoLuaMulti<'lua>,
        R: mlua::FromLuaMulti<'lua>,
    {
        Ok(self
            .ctx
            .globals()
            .get::<&str, mlua::Function<'lua>>(func_name.trim())?
            .call(args)?)
    }

    pub fn set_fn<'lua, A, R, F>(&'lua self, name: &str, func: F) -> anyhow::Result<()>
    where
        A: mlua::FromLuaMulti<'lua>,
        R: mlua::IntoLuaMulti<'lua>,
        F: Fn(&'lua mlua::Lua, A) -> mlua::Result<R> + 'static,
    {
        let func = self.ctx.create_function(func)?;
        self.ctx.globals().set(name, func)?;
        Ok(())
    }
}
