--speed:0,0

---$embed
local curves = require("common")

local function smooth_edge_value(start_value, end_value, t, is_first_edge, is_last_edge)
	if not is_first_edge and not is_last_edge then
		return start_value + (end_value - start_value) * t
	end

	local average = (start_value + end_value) * 0.5
	local left = is_first_edge and average or start_value
	local right = is_last_edge and average or end_value
	local s = 1.0 - t

	return (left * t * 3.0 + s * start_value) * s * s + (right * s * 3.0 + t * end_value) * t * t
end

local function rotation_linear_value(ctx, values)
	local period = ctx.rotation_period or ctx.angle_period or 360.0
	return curves.linear_value(ctx, curves.build_rotation_series(values, period))
end

local function linear_rotate_group_value(ctx, axes)
	local segment, t = curves.resolve_segment(ctx, #axes[1])
	local flags = curves.get_flags(ctx)
	local smooth_t = smooth_edge_value(
		0.0,
		1.0,
		t,
		flags.double_first and segment == 0,
		flags.double_last and segment == #axes[1] - 2
	)
	local order = ctx.rotation_order or "xyz"
	local q0 = curves.euler_quat_at(axes, segment + 1, order)
	local q1 = curves.euler_quat_at(axes, segment + 2, order)
	return curves.rotation_component_from_quat(ctx, curves.quat_slerp(q0, q1, smooth_t))
end

local ctx = curves.make_ctx()
local axes = curves.rotation_axes(ctx)
if axes then
	return linear_rotate_group_value(ctx, axes)
end

return rotation_linear_value(ctx, curves.normalize_values(ctx.values or {}, ctx.divisor))
