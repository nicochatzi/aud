function on_start()
    log("on_start")
end

function on_discover(device_names)
    log("on_discover : [ " .. table.concat(device_names, ", ") .. " ]")
end

function on_connect(device_name)
    log("on_connect : " .. device_name)
end

function on_midi(device_name, bytes)
    log("on_midi : " .. device_name .. " : " .. #bytes .. " bytes")
    return true
end

function on_stop()
    log("on_stop")
end
