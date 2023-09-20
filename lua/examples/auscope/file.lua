local file = nil
local filepath = 'out/midimon.mid'

function on_start()
	if file then io.close(file) end

	file = io.open(filepath, 'wb')
	io.output(file)

	alert("opening file : " .. filepath)
end

function on_audio(device_name, buffer)
	if file then
		for _, channel in ipairs(buffer) do
			for _, sample in ipairs(buffer) do
				file:write(sample)
			end
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
