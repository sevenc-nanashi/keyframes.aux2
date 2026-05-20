--speed:0,0
--timecontrol

---$embed
local curves = require("common")

local ctx = curves.make_ctx()
local axes = curves.collect_axes(ctx)
local segment, t, lengths = curves.weighted_segment(ctx, axes)

return curves.interpolation_value({
	values = ctx.values,
	divisor = ctx.divisor,
	segment = segment,
	local_t = t,
	double_first = ctx.double_first,
	double_last = ctx.double_last,
	edge_flags = ctx.edge_flags,
}, curves.normalize_values(ctx.values or {}, ctx.divisor), lengths)
