-- List the commands available for a given device
--
-- @param device string: The name of the MIDI device for which the SysEx commands are intended.
-- @return table: A table where the keys are command names and the values are a table of named arguments
function list_commands(device) end

-- Build a SysEx command
--
-- @param device string: The name of the MIDI device for which the SysEx commands are intended.
-- @param command string: The name of the SysEx command to build
-- @param args table: A table of named arguments that will be encoded in the SysEx command
function build_command(device, command, args) end

-- Parse a SysEx response
--
-- @param device string: The name of the MIDI device that sent this SysEx message.
-- @param bytes table: A table of bytes representing the raw SysEx message.
-- @return string: A string representation of the parsed SysEx message.
function parse_sysex(device, bytes) end
