local M = {}

M.headers = {
    sysex_start = 0xF0,
    sysex_end = 0xF7,
    note_on = 0x90,
    note_off = 0x80,
    poly_pressure = 0xA0,
    controller = 0xB0,
    program_change = 0xC0,
    channel_pressure = 0xD0,
    pitch_bend = 0xE0,
}

function M.parse_header(bytes)
    for header in M.headers do
        if bytes[0] & header ~= 0 then
            return header
        end
    end

    return nil
end

return M
