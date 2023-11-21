local single_byte_manufacturer_names = {
    [0x01] = "Sequential",
    [0x02] = "IDP",
    [0x03] = "Octave-Plateau / Voyetra",
    [0x04] = "Moog Music",
    [0x05] = "Passport Designs",
    [0x06] = "Lexicon",
    [0x07] = "Kurzweil",
    [0x0F] = "Ensoniq",
    [0x10] = "Oberheim",
    [0x11] = "Apple Computer",
    [0x13] = "Digidesign",
    [0x15] = "JL Cooper",
    [0x18] = "E-mu",
    [0x1C] = "Eventide",
    [0x22] = "Synthaxe",
    [0x27] = "Jellinghaus",
    [0x29] = "PPG",
    [0x2D] = "Hinton Instruments",
    [0x2F] = "Elka",
    [0x31] = "Viscount",
    [0x33] = "Clavia",
    [0x38] = "Simmons",
    [0x3A] = "Steinberg",
    [0x3E] = "Waldorf",
    [0x3F] = "Quasimidi",
    [0x40] = "Kawai",
    [0x41] = "Roland",
    [0x42] = "Korg",
    [0x43] = "Yamaha",
    [0x44] = "Casio",
    [0x47] = "Akai",
    [0x4C] = "Sony",
    [0x52] = "Zoom",
}

local triple_byte_manufacturer_names = {
    [0x000009] = "New England Digital",
    [0x000016] = "Opcode",
    [0x00001B] = "Peavey",
    [0x00001C] = "360 Systems",
    [0x00001F] = "Zeta",
    [0x00002F] = "Encore Electronics",
    [0x00003B] = "MOTU",
    [0x000041] = "Microsoft",
    [0x00004D] = "Studio Electronics",
    [0x000105] = "M-Audio",
    [0x000121] = "Cakewalk",
    [0x000137] = "Roger Linn Design",
    [0x00013F] = "Numark / Alesis",
    [0x00014D] = "Open Labs",
    [0x000172] = "Kilpatrick Audio",
    [0x000177] = "Nektar",
    [0x000214] = "Intellijel",
    [0x00021F] = "Madrona Labs",
    [0x000226] = "Electro-Harmonix",
    [0x002013] = "Kenton",
    [0x00201A] = "Fatar / Studiologic",
    [0x00201F] = "TC Electronic",
    [0x002029] = "Novation",
    [0x002032] = "Behringer",
    [0x002033] = "Access Music",
    [0x00203A] = "Propellorhead",
    [0x00203B] = "Red Sound",
    [0x00204D] = "Vermona",
    [0x002050] = "Hartmann",
    [0x002052] = "Analogue Systems",
    [0x00205F] = "Sequentix",
    [0x002069] = "Elby Designs",
    [0x00206B] = "Arturia",
    [0x002076] = "Teenage Engineering",
    [0x002102] = "Mutable Instruments",
    [0x002107] = "Modal Electronics",
    [0x002109] = "Native Instruments",
    [0x002110] = "ROLI",
    [0x00211A] = "IK Multimedia",
    [0x002127] = "Expert Sleepers",
    [0x002135] = "Dreadbox",
    [0x002141] = "Marienberg",
}


local function for_all_manufacturers(func)
    for list in { single_byte_manufacturer_names, triple_byte_manufacturer_names } do
        for id, manufacturer in pairs(list) do
            func(id, manufacturer)
        end
    end

    return nil
end

local M = {}

function M.find_manufacturer_name_from_id(id)
    return for_all_manufacturers(function(manufacturer_id, manufacturer)
        if manufacturer_id == id then return manufacturer end
    end
    )
end

function M.find_manufacturer_id_from_name(name)
    return for_all_manufacturers(function(id, manufacturer)
        if manufacturer == name then return id end
    end
    )
end

return M
