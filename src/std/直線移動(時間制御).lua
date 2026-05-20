--speed:0,0
--timecontrol

---$embed
local curves = require("common")

local num = obj.getpoint("num")
local values = {}
for i = 0, num - 1 do
	values[i + 1] = obj.getpoint(i)
end

local link_index, link_count = obj.getpoint("link")
link_index = link_index or 0
link_count = link_count or 1

local linked_values = nil
if link_count > 1 then
	linked_values = {}
	for axis = 0, link_count - 1 do
		local axis_values = {}
		for i = 0, num - 1 do
			axis_values[i + 1] = obj.getpoint(i, axis - link_index)
		end
		linked_values[axis + 1] = axis_values
	end
end

local t = num <= 1 and 0.0 or math.max(0.0, math.min(1.0, obj.getpoint("index") / (num - 1)))
local ok, timecontrol_value = pcall(obj.getpoint, "timecontrol", "value")
if ok and timecontrol_value then
	t = timecontrol_value
end

local axes = curves.collect_axes(values, linked_values)
local segment, local_t = curves.weighted_segment(axes, t, obj.getpoint("accelerate"), obj.getpoint("decelerate"))

return curves.linear_value(values, segment, local_t, nil, obj.getpoint("accelerate"), obj.getpoint("decelerate"))
