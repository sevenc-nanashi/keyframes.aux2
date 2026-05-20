--speed:0,0

---$embed
local curves = require("common")

local index, ratio = math.modf(obj.getpoint("index"))
local num = obj.getpoint("num")
local values = {}
for i = 0, num - 1 do
	values[i + 1] = obj.getpoint(i)
end

local segment = curves.resolve_segment(#values, index, ratio, nil)
return values[segment + 1] or values[#values] or 0.0
