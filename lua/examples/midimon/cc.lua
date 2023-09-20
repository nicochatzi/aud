function on_midi(_, bytes)
	if bytes == nil or #bytes < 1 then
		return true
	end

	local controller = 0xB0
	return (bytes[1] & 0xF0) == controller
end
