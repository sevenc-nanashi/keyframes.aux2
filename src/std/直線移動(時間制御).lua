--speed:0,0
--timecontrol

---$embed
local curves = require("common")

local index, ratio = math.modf(obj.getpoint("timecontrol", "index"))
local num = obj.getpoint("num")
local values = {}
for i = 0, num - 1 do
	values[i + 1] = obj.getpoint(i)
end

return curves.linear_value(values, index, ratio, nil, obj.getpoint("accelerate"), obj.getpoint("decelerate"))
