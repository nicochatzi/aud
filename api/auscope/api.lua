-- [ functions defined in your script ]

-- Called when the `aud` is starting
--
-- @param number: App timeout in millis, after which the app auto-closes. 0 or nil is never.
function on_start() end

-- Called when audio device list is updated.
--
-- @param device_names string list: Names of the discovered audio devices
function on_discover(device_names) end

-- Called when a audio device is connected
--
-- @param device_name string: Name of the audio device we've just connected to
function on_connect(device_name) end

-- Called when audio is received.
--
-- @param device_name string: Name of the device sending the audio
-- @param buffer table: A table of tables or numbers, i.e. the multi-channel audio buffer
function on_audio(device_name, buffer) end

-- Called when `aud` is stopping
function on_stop() end
