<h1 align="center"><code>aud</code></h1>

<p align="center">
scriptable <code>aud</code>io terminal tools
</p>

<p align="center">
<img src="./vhs/out/scope_loop.gif">
</p>

🧱 `Requires`: [Rust](https://www.rust-lang.org/tools/install) and [Just](https://github.com/casey/just)

🌶️ `Scriptable`: in [Lua](https://www.lua.org/start.html), with `hooks`, `hot-reloading` and `sandboxed panics`

🔨 `Install`: `just install <INSTALL_DIR>`: build and install `aud` on your system

💻 `Contribute`: `just setup`: setup development environment for this project

<h2 align="center"><code>usage</code></h2>

After installing, you can generate and install terminal auto-completions scripts.

![aud](./vhs/out/aud.gif)

<h2 align="center"><code>commands</code></h2>

### `midimon`

Scriptable MIDI Monitor.

![midimon](./vhs/out/midimon.gif)

### `auscope`

Scriptable Audio Oscilloscope.

By default `auscope` lists the host machine's audio devices.
`audlib` can integrated in other applications (Rust or through C-FFI)
to generate sources and send them over UDP to an `auscope` instance.

![auscope](./vhs/out/auscope.gif)

### `derlink`

Simple Ableton Link Client.

![derlink](./vhs/out/derlink.gif)
