--speed:0,0

---$embed
local curves = require("common")

local EPS = 1.0e-12
local ROTATION_CONTROL_BLEND = 0.42500001192092896

local function atan2(y, x)
	if math.atan2 then
		return math.atan2(y, x)
	end
	return math.atan(y, x)
end

local function quat_normalize(quat)
	local length = math.sqrt(quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3] + quat[4] * quat[4])
	if length <= EPS then
		return { 0.0, 0.0, 0.0, 1.0 }
	end
	return {
		quat[1] / length,
		quat[2] / length,
		quat[3] / length,
		quat[4] / length,
	}
end

local function quat_mul(a, b)
	return {
		a[4] * b[1] + a[1] * b[4] + a[2] * b[3] - a[3] * b[2],
		a[4] * b[2] - a[1] * b[3] + a[2] * b[4] + a[3] * b[1],
		a[4] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[4],
		a[4] * b[4] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
	}
end

local function quat_add(a, b)
	return {
		a[1] + b[1],
		a[2] + b[2],
		a[3] + b[3],
		a[4] + b[4],
	}
end

local function quat_scale(quat, scale)
	return {
		quat[1] * scale,
		quat[2] * scale,
		quat[3] * scale,
		quat[4] * scale,
	}
end

local function quat_align(reference, quat)
	if reference[1] * quat[1] + reference[2] * quat[2] + reference[3] * quat[3] + reference[4] * quat[4] < 0.0 then
		return { -quat[1], -quat[2], -quat[3], -quat[4] }
	end
	return quat
end

local function quat_inverse(quat)
	quat = quat_normalize(quat)
	return { -quat[1], -quat[2], -quat[3], quat[4] }
end

local function quat_log(quat)
	quat = quat_normalize(quat)
	local v_len = math.sqrt(quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3])
	if v_len <= EPS then
		return { 0.0, 0.0, 0.0, 0.0 }
	end
	local angle = atan2(v_len, quat[4])
	local scale = angle / v_len
	return { quat[1] * scale, quat[2] * scale, quat[3] * scale, 0.0 }
end

local function quat_exp(quat)
	local v_len = math.sqrt(quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3])
	local s = math.sin(v_len)
	if v_len <= EPS then
		return { quat[1], quat[2], quat[3], math.cos(v_len) }
	end
	local scale = s / v_len
	return {
		quat[1] * scale,
		quat[2] * scale,
		quat[3] * scale,
		math.cos(v_len),
	}
end

local function quat_squad_control(prev_q, cur_q, next_q)
	local inv_cur = quat_inverse(cur_q)
	local log1 = quat_log(quat_mul(inv_cur, prev_q))
	local log2 = quat_log(quat_mul(inv_cur, next_q))
	local blend = quat_scale(quat_add(log1, log2), -0.25)
	return quat_normalize(quat_mul(cur_q, quat_exp(blend)))
end

local function rotation_interpolation_value(ctx, values, lengths)
	local period = ctx.rotation_period or ctx.angle_period or 360.0
	return curves.interpolation_value(ctx, curves.build_rotation_series(values, period), lengths)
end

local function interpolation_rotate_group_value(ctx, axes)
	local segment, t = curves.resolve_segment(ctx, #axes[1])
	local order = ctx.rotation_order or "xyz"
	local q_prev = curves.euler_quat_at(axes, segment, order)
	local q_cur = curves.euler_quat_at(axes, segment + 1, order)
	local q_next = quat_align(q_cur, curves.euler_quat_at(axes, segment + 2, order))
	q_prev = quat_align(q_cur, q_prev)
	local q_after = quat_align(q_next, curves.euler_quat_at(axes, segment + 3, order))

	local control_cur = quat_squad_control(q_prev, q_cur, q_next)
	local control_next = quat_squad_control(q_cur, q_next, q_after)
	local curve_cur = curves.quat_slerp(q_cur, control_cur, ROTATION_CONTROL_BLEND)
	local curve_next = curves.quat_slerp(q_next, control_next, ROTATION_CONTROL_BLEND)
	local curve = curves.quat_slerp(curve_cur, curve_next, t)
	local linear = curves.quat_slerp(q_cur, q_next, t)
	local blend = 4.0 * t * (1.0 - t)
	return curves.rotation_component_from_quat(ctx, curves.quat_slerp(linear, curve, blend))
end

local ctx = curves.make_ctx()
local rotation_group = curves.rotation_axes(ctx)
if rotation_group then
	return interpolation_rotate_group_value(ctx, rotation_group)
end

local axes = curves.collect_axes(ctx)
local lengths = curves.segment_lengths(axes, curves.get_flags(ctx))
return rotation_interpolation_value(ctx, curves.normalize_values(ctx.values or {}, ctx.divisor), lengths)
