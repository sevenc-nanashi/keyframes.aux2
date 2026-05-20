--speed:0,0
--twopoint

---$embed
local curves = require("common")

local ctx = curves.make_ctx()
local values = curves.normalize_values(ctx.values or {}, ctx.divisor)
if #values == 0 then
	return 0.0
end
if #values == 1 then
	return values[1]
end

return values[1] + values[2] * (ctx.t or 0.0)
