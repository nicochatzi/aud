function on_start()
    alert("on_start")
end

function on_discover(device_names)
    alert("on_discover:" .. table.concat(device_names, ","))
end

function on_connect(device_name)
    alert("on_connect:" .. device_name)
end

function on_midi(device_name, bytes)
    alert("on_midi:" .. device_name .. ":" .. table.concat(bytes, ","))
end

function on_audio(device_name, buffer)
    alert("on_audio:" .. device_name .. ":" .. table.concat(buffer, ","))
end

function on_stop()
    alert("on_stop")
end
