<h1 align="center"><code>midimon</code></h1>
<p align="center">Scriptable MIDI Monitor</p>

![midimon](../res/out/midimon.gif)

## Usage

Run `aud midimon` to start the MIDI monitor.
By default it will log to `~/.aud/log/aud.log`.

If it finds a script directory, you can select
a script to hook into the monitor.

The scripts can optionally provided any or all of
these [these functions](../lua/api/midimon/api.lua), and
can call into the running application with
[these functions](../lua/api/midimon/docs.lua).

Script examples can be found [here](../lua/examples/midimon/).
