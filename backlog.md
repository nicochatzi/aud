## `midimon`

- [ ] Make messages scrollable - like the selector widget
- [ ] With scrollable messages, a new pane could show details (maybe a popup?)
    - Timestamp -> precise time, time between this message the previous and the next in `ms`
    - Render sysex by parsing it with a user provided script
- [ ] Some sort of filtering!

## `auscope`

- [ ] App should append buffers, ui should consume as much as will fit on screen
- [ ] Add flag to watch input or output signals
- [ ] Add channel / bus selector
    - Chart 1 or 2 signals max
    - 1 pane to select the device, second to select the channel or group (stereo)
- [ ] Peak detection
- [ ] Discontinuity detection

## `derlink`

- [ ] Get information on / list peers?

## future

### apps

- `latency`: measure round trip audio or midi to audio latency
    - top left pane: MIDI output port
    - top right pane: audio input port
    - bottom pane: latency state
- `convert`: common audio value conversions (midi to freq, cycles per sample...)
    - left pane: select converter
    - right pane: interpreter with a scrollable history of previous commands
- `signals`: common audio signals (sine, sweep, loops...)
    - left pane: select signal
    - right pane: enter signal params
    - bottom pane: scope
- `lutsgen`: generate lookup tables
    - same sort of layout as `signals`
    - needs ability to copy
    - should be able to graph
- `oscview`: view incoming OSC messages on a given port
- `mpeview`: view incoming MIDI MPE messages (grouped controls/notes per channel)
- `midiseq`: random midi note (and cc?) generator

### features

- frame rate option at start-up for all apps
- ability to copy to clipboard
- make stateful widgets searchable
- stateful tree like widget. can be used for filtering. 
