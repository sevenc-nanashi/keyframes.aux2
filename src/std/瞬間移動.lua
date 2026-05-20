--speed:0,0

---$embed
local curves = require("common")

local ctx = curves.make_ctx()
local values = curves.normalize_values(ctx.values or {}, ctx.divisor)
local segment = curves.resolve_segment(ctx, #values)
return values[segment + 1] or values[#values] or 0.0
