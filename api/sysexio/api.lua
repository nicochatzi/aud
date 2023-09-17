--- commands function
--
-- This function returns a table (map) of SysEx commands
-- specific to a given device name.
--
-- @param device string: The name of the MIDI device for which the SysEx commands are intended.
-- @return table: A table where the keys are command names and the values are tables of bytes for the corresponding SysEx messages.
function list_commands(device) end

--- parse function
--
-- This function parses the raw SysEx bytes and returns a human-readable
-- string representation for the CLI to pretty-print.
--
-- @param device string: The name of the MIDI device that sent this SysEx message.
-- @param bytes table: A table of bytes representing the raw SysEx message.
-- @return string: A string representation of the parsed SysEx message.
function parse_sysex(device, bytes) end
