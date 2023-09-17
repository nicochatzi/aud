-- [ functions defined in your script ]

-- Called when the `aud` is starting
--
-- @param number: App timeout in millis, after which the app auto-closes. 0 or nil is never.
function on_start() end

-- Called when MIDI device list is updated.
--
-- @param device_names string list: Names of the discovered MIDI devices
function on_discover(device_names) end

-- Called when a MIDI connection is made
--
-- @param device_name string: Name of the MIDI device we've just connected to
function on_connect(device_name) end

-- Called when MIDI bytes are received.
--
-- @param device_name string: Name of the MIDI device sending this MIDI
-- @param bytes table: A table of bytes representing the raw MIDI message.
-- @return bool: Should this message be displayed?
function on_midi(device_name, bytes) end

-- Called at frame rate
function on_tick() end

-- Called when `aud` is stopping
function on_stop() end
