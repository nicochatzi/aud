local file = nil

function on_start()
	if file then io.close(file) end

	file = io.open('out/midimon.mid', 'wb')
	io.output(file)

	log("starting to write")
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
		log("done writing")
	end

	file = nil
end
