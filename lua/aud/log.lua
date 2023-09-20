local M = {}

function M.print_table_or_user_data(t, indent)
	indent = indent or 0
	local t_type = type(t)

	if t_type == "table" then
		for k, v in pairs(t) do
			if type(v) == "table" or type(v) == "userdata" then
				print(string.rep("  ", indent) .. k .. ":")
				M.print_table_or_user_data(v, indent + 1)
			else
				print(string.rep("  ", indent) .. k .. ": " .. tostring(v))
			end
		end
	elseif t_type == "userdata" then
		local meta = getmetatable(t)
		if meta and meta.__tostring then
			print(string.rep("  ", indent) .. tostring(t))
		else
			print(string.rep("  ", indent) .. "userdata")
		end
	else
		print(string.rep("  ", indent) .. tostring(t))
	end
end

return M
