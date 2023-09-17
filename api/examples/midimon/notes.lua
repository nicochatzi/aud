function on_midi(_, bytes)
	if bytes == nil or #bytes < 1 then
		return true
	end

	local note_on = 0x90
	local note_off = 0x80

	for _, header in pairs({ note_on, note_off }) do
		if (bytes[1] & 0xF0) == header then return true end
	end

	return false
end
