--speed:0,0

---$embed
local curves = require("common")

local ctx = curves.make_ctx()
return curves.linear_value(ctx, curves.normalize_values(ctx.values or {}, ctx.divisor))
