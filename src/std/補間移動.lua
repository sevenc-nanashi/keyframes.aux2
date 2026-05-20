--speed:0,0

---$embed
local curves = require("common")

local ctx = curves.make_ctx()
local values = curves.normalize_values(ctx.values or {}, ctx.divisor)
local axes = curves.collect_axes(ctx)
local lengths = curves.segment_lengths(axes, curves.get_flags(ctx))

return curves.interpolation_value(ctx, values, lengths)
