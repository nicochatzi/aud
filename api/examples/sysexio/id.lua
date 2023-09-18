function list_commands(device)
	local common_commands = {
		["Id Request"] = { 0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7 }
	}

	local device_commands = {
		["Device A"] = {
			["ResetDevice"] = { 0xF0, 0x7E, 0x7F, 0x09, 0x01, 0xF7 },
			common_commands
		},
		["Device B"] = {
			["GetFirmwareVersion"] = { 0xF0, 0x00, 0x01, 0x73, 0x60, 0xF7 },
			common_commands
		}
	}

	return device_commands[device] or common_commands
end
