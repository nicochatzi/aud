local file = nil
local filepath = 'out/midimon.mid'

function on_start()
	if file then io.close(file) end

	file = io.open(filepath, 'wb')
	io.output(file)

	alert("opening file : " .. filepath)
end

function on_midi(device_name, bytes)
	if file then
		for _, byte in ipairs(bytes) do
			file:write(string.char(byte))
		end
	end

	return true
end

function on_stop()
	if file then
		file:close()
		alert("wrote file : " .. filepath)
	end

	file = nil
end
