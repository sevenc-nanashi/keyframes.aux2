--speed:0,0

---$embed
local curves = require("common")

local RAND_MAX = 2147483647

local function random_unit(seed, index)
	local frame = obj.frame or 0
	return obj.rand(0, RAND_MAX, seed + index * 65537, frame) / RAND_MAX
end

local function random_move_value(ctx, values)
	if #values == 0 then
		return 0.0
	end
	if #values == 1 then
		return values[1]
	end

	local segment, t = curves.resolve_segment(ctx, #values)
	local base = values[1]
	local span = values[#values] - base
	local seed = ctx.seed or 0

	local function point(index)
		return base + random_unit(seed, index) * span
	end

	local p0 = point(segment)
	local p1 = point(segment + 1)
	local p2 = point(segment + 2)
	local p3 = point(segment + 3)

	return curves.catmull_rom(p0, p1, p2, p3, 1.0, 1.0, 1.0, t)
end

local ctx = curves.make_ctx()
return random_move_value(ctx, curves.normalize_values(ctx.values or {}, ctx.divisor))
