--	[ functions provided by `aud` ]

-- Write to the `aud` log file
function log(message) end

-- Connect to the specified MIDI device
function connect(device_name) end

-- Send an alert to `aud`
--
-- @return string: Alert message
function alert(message) end

-- Pause the stream
function pause() end

-- Resume streaming
function resume() end

-- Request to stop the application
function stop() end
