local M = {}

local EPS = 1.0e-12
local HALF = 0.5
local ROTATION_CONTROL_BLEND = 0.42500001192092896
local RAND_MAX = 2147483647

local function clamp(value, min_value, max_value)
	if value < min_value then
		return min_value
	end
	if value > max_value then
		return max_value
	end
	return value
end

local function smooth_edge_value(start_value, end_value, t, is_first_edge, is_last_edge)
	if not is_first_edge and not is_last_edge then
		return start_value + (end_value - start_value) * t
	end

	local average = (start_value + end_value) * HALF
	local left = is_first_edge and average or start_value
	local right = is_last_edge and average or end_value
	local s = 1.0 - t

	return (left * t * 3.0 + s * start_value) * s * s + (right * s * 3.0 + t * end_value) * t * t
end

local function get_flags(ctx)
	local flags = ctx.edge_flags or {}
	return {
		double_first = ctx.double_first or flags.double_first or false,
		double_last = ctx.double_last or flags.double_last or false,
	}
end

local function resolve_segment(ctx, point_count)
	if point_count <= 1 then
		return 0, 0.0
	end

	if ctx.segment ~= nil then
		return clamp(ctx.segment, 0, point_count - 2), clamp(ctx.local_t or ctx.t or 0.0, 0.0, 1.0)
	end

	local t = clamp(ctx.t or 0.0, 0.0, 1.0)
	local scaled = t * (point_count - 1)
	local segment = math.min(point_count - 2, math.floor(scaled))
	return segment, scaled - segment
end

local function normalize_values(values, divisor)
	divisor = divisor or 1.0
	local out = {}
	for i = 1, #values do
		out[i] = values[i] / divisor
	end
	return out
end

local function deg_to_rad(value)
	return value * math.pi / 180.0
end

local function rad_to_deg(value)
	return value * 180.0 / math.pi
end

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

local function quat_conjugate(quat)
	return { -quat[1], -quat[2], -quat[3], quat[4] }
end

local function quat_dot(a, b)
	return a[1] * b[1] + a[2] * b[2] + a[3] * b[3] + a[4] * b[4]
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

local function quat_negate(quat)
	return { -quat[1], -quat[2], -quat[3], -quat[4] }
end

local function quat_align(reference, quat)
	if quat_dot(reference, quat) < 0.0 then
		return quat_negate(quat)
	end
	return quat
end

local function quat_inverse(quat)
	return quat_conjugate(quat_normalize(quat))
end

local function axis_angle_quat(axis, angle_rad)
	local half = angle_rad * HALF
	local s = math.sin(half)
	if axis == "x" then
		return { s, 0.0, 0.0, math.cos(half) }
	end
	if axis == "y" then
		return { 0.0, s, 0.0, math.cos(half) }
	end
	return { 0.0, 0.0, s, math.cos(half) }
end

local function quat_from_euler_xyz(rx, ry, rz, order)
	order = order or "xyz"
	local quats = {
		x = axis_angle_quat("x", deg_to_rad(rx)),
		y = axis_angle_quat("y", deg_to_rad(ry)),
		z = axis_angle_quat("z", deg_to_rad(rz)),
	}

	local result = { 0.0, 0.0, 0.0, 1.0 }
	for i = #order, 1, -1 do
		local axis = order:sub(i, i)
		result = quat_mul(quats[axis], result)
	end
	return quat_normalize(result)
end

local function quat_to_euler_xyz(quat, order)
	order = order or "xyz"
	quat = quat_normalize(quat)

	if order ~= "xyz" then
		order = "xyz"
	end

	local x, y, z, w = quat[1], quat[2], quat[3], quat[4]

	local sinr_cosp = 2.0 * (w * x - y * z)
	local cosr_cosp = 1.0 - 2.0 * (x * x + y * y)
	local rx = atan2(sinr_cosp, cosr_cosp)

	local sinp = 2.0 * (w * y + z * x)
	local ry
	if math.abs(sinp) >= 1.0 then
		ry = (sinp >= 0.0 and 1.0 or -1.0) * (math.pi * HALF)
	else
		ry = math.asin(sinp)
	end

	local siny_cosp = 2.0 * (w * z - x * y)
	local cosy_cosp = 1.0 - 2.0 * (y * y + z * z)
	local rz = atan2(siny_cosp, cosy_cosp)

	return {
		rad_to_deg(rx),
		rad_to_deg(ry),
		rad_to_deg(rz),
	}
end

local function quat_slerp(a, b, t)
	local dot = quat_dot(a, b)
	if dot < 0.0 then
		b = quat_negate(b)
		dot = -dot
	end

	if dot > 0.9995 then
		return quat_normalize(quat_add(quat_scale(a, 1.0 - t), quat_scale(b, t)))
	end

	local theta_0 = math.acos(clamp(dot, -1.0, 1.0))
	local theta = theta_0 * t
	local sin_theta = math.sin(theta)
	local sin_theta_0 = math.sin(theta_0)

	local s0 = math.cos(theta) - dot * sin_theta / sin_theta_0
	local s1 = sin_theta / sin_theta_0
	return quat_add(quat_scale(a, s0), quat_scale(b, s1))
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

local function build_rotation_series(values, period)
	if #values == 0 then
		return {}
	end

	local out = { values[1] }
	for i = 2, #values do
		local delta = (values[i] - out[i - 1]) % period
		if delta > period * HALF then
			delta = delta - period
		end
		out[i] = out[i - 1] + delta
	end
	return out
end

local function collect_axes(ctx)
	if ctx.linked_values then
		local axes = {}
		for _, values in ipairs(ctx.linked_values) do
			axes[#axes + 1] = normalize_values(values, ctx.divisor)
		end
		if #axes > 0 then
			return axes
		end
	end

	return { normalize_values(ctx.values or {}, ctx.divisor) }
end

local function rotation_axes(ctx)
	if not ctx.linked_values or #ctx.linked_values ~= 3 then
		return nil
	end
	return {
		normalize_values(ctx.linked_values[1], ctx.divisor),
		normalize_values(ctx.linked_values[2], ctx.divisor),
		normalize_values(ctx.linked_values[3], ctx.divisor),
	}
end

local function euler_quat_at(axes, index, order)
	local max_index = #axes[1]
	local i = clamp(index, 1, max_index)
	return quat_from_euler_xyz(axes[1][i], axes[2][i], axes[3][i], order)
end

local function rotation_component_from_quat(ctx, quat)
	local euler = quat_to_euler_xyz(quat, ctx.rotation_order or "xyz")
	local axis_index = clamp(ctx.axis_index or ctx.component or 1, 1, 3)
	return euler[axis_index]
end

local function linear_rotate_group_value(ctx, axes)
	local segment, t = resolve_segment(ctx, #axes[1])
	local flags = get_flags(ctx)
	local smooth_t = smooth_edge_value(
		0.0,
		1.0,
		t,
		flags.double_first and segment == 0,
		flags.double_last and segment == #axes[1] - 2
	)
	local order = ctx.rotation_order or "xyz"
	local q0 = euler_quat_at(axes, segment + 1, order)
	local q1 = euler_quat_at(axes, segment + 2, order)
	return rotation_component_from_quat(ctx, quat_slerp(q0, q1, smooth_t))
end

local function interpolation_rotate_group_value(ctx, axes)
	local segment, t = resolve_segment(ctx, #axes[1])
	local order = ctx.rotation_order or "xyz"
	local q_prev = euler_quat_at(axes, segment, order)
	local q_cur = euler_quat_at(axes, segment + 1, order)
	local q_next = quat_align(q_cur, euler_quat_at(axes, segment + 2, order))
	q_prev = quat_align(q_cur, q_prev)
	local q_after = quat_align(q_next, euler_quat_at(axes, segment + 3, order))

	local control_cur = quat_squad_control(q_prev, q_cur, q_next)
	local control_next = quat_squad_control(q_cur, q_next, q_after)
	local curve_cur = quat_slerp(q_cur, control_cur, ROTATION_CONTROL_BLEND)
	local curve_next = quat_slerp(q_next, control_next, ROTATION_CONTROL_BLEND)
	local curve = quat_slerp(curve_cur, curve_next, t)
	local linear = quat_slerp(q_cur, q_next, t)
	local blend = 4.0 * t * (1.0 - t)
	return rotation_component_from_quat(ctx, quat_slerp(linear, curve, blend))
end

local function segment_lengths(axes, flags)
	local point_count = #axes[1]
	if point_count <= 1 then
		return {}
	end

	local lengths = {}
	for i = 1, point_count - 1 do
		local sum = 0.0
		for _, axis in ipairs(axes) do
			local delta = axis[i + 1] - axis[i]
			sum = sum + delta * delta
		end
		lengths[i] = math.sqrt(sum)
	end

	if #lengths > 0 then
		if flags.double_first then
			lengths[1] = lengths[1] * 2.0
		end
		if flags.double_last then
			lengths[#lengths] = lengths[#lengths] * 2.0
		end
	end

	return lengths
end

local function weighted_segment(ctx, axes)
	local flags = get_flags(ctx)
	local lengths = segment_lengths(axes, flags)
	if #lengths == 0 then
		return 0, clamp(ctx.t or 0.0, 0.0, 1.0), lengths
	end

	local total = 0.0
	for i = 1, #lengths do
		total = total + lengths[i]
	end
	if total <= EPS then
		return resolve_segment(ctx, #axes[1]), lengths
	end

	local rest = clamp(ctx.t or 0.0, 0.0, 1.0) * total
	for i = 1, #lengths do
		if rest <= lengths[i] then
			return i - 1, lengths[i] <= EPS and 0.0 or rest / lengths[i], lengths
		end
		rest = rest - lengths[i]
	end

	return #lengths - 1, 1.0, lengths
end

local function catmull_rom(start_prev, start_value, end_value, end_next, len_prev, len_cur, len_next, t)
	len_prev = math.max(len_prev or 1.0, EPS)
	len_cur = math.max(len_cur or 1.0, EPS)
	len_next = math.max(len_next or 1.0, EPS)

	local m0 = ((end_value - start_prev) / (len_prev + len_cur)) * len_cur * HALF
	local m1 = ((end_next - start_value) / (len_cur + len_next)) * len_cur * HALF
	local s = 1.0 - t

	return ((start_value + m0) * t * 3.0 + s * start_value) * s * s
		+ ((end_value - m1) * s * 3.0 + t * end_value) * t * t
end

local function linear_value(ctx, values)
	local segment, t = resolve_segment(ctx, #values)
	if #values == 0 then
		return 0.0
	end
	if #values == 1 then
		return values[1]
	end

	local flags = get_flags(ctx)
	return smooth_edge_value(
		values[segment + 1],
		values[segment + 2],
		t,
		flags.double_first and segment == 0,
		flags.double_last and segment == #values - 2
	)
end

local function interpolation_value(ctx, values, lengths)
	local segment = ctx.segment
	local t = ctx.local_t
	if segment == nil or t == nil then
		segment, t = resolve_segment(ctx, #values)
	end

	if #values == 0 then
		return 0.0
	end
	if #values == 1 then
		return values[1]
	end

	local i = segment + 1
	local p0 = values[i]
	local p1 = values[i + 1]
	local pm1 = values[math.max(i - 1, 1)]
	local p2 = values[math.min(i + 2, #values)]

	local flags = get_flags(ctx)
	if i == 1 and not flags.double_first then
		pm1 = (2.0 * p0 - p1) + HALF * p2
	end
	if i == #values - 1 and not flags.double_last then
		p2 = (2.0 * p1 - p0) + HALF * pm1
	end

	local len_prev = lengths and lengths[math.max(i - 1, 1)] or 1.0
	local len_cur = lengths and lengths[i] or 1.0
	local len_next = lengths and lengths[math.min(i + 1, #lengths)] or len_cur

	return catmull_rom(pm1, p0, p1, p2, len_prev, len_cur, len_next, t)
end

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

	local segment, _t = resolve_segment(ctx, #values)
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

	return catmull_rom(p0, p1, p2, p3, 1.0, 1.0, 1.0, _t)
end

local function speed_move_value(ctx, values)
	if #values == 0 then
		return 0.0
	end
	if #values == 1 then
		return values[1]
	end
	return values[1] + values[2] * (ctx.t or 0.0)
end

local function teleport_value(ctx, values)
	local segment = resolve_segment(ctx, #values)
	return values[segment + 1] or values[#values] or 0.0
end

local function rotation_linear_value(ctx, values)
	local period = ctx.rotation_period or ctx.angle_period or 360.0
	return linear_value(ctx, build_rotation_series(values, period))
end

local function rotation_interpolation_value(ctx, values, lengths)
	local period = ctx.rotation_period or ctx.angle_period or 360.0
	return interpolation_value(ctx, build_rotation_series(values, period), lengths)
end

local function make_linked_values(point_count, link_index, link_count)
	if not link_count or link_count <= 1 then
		return nil
	end

	local linked_values = {}
	for axis = 0, link_count - 1 do
		local values = {}
		for i = 0, point_count - 1 do
			values[i + 1] = obj.getpoint(i, axis - link_index)
		end
		linked_values[axis + 1] = values
	end
	return linked_values
end

function M.make_ctx()
	local index_value = obj.getpoint("index")
	local segment, local_t = math.modf(index_value)
	local point_count = obj.getpoint("num")
	local values = {}
	for i = 0, point_count - 1 do
		values[i + 1] = obj.getpoint(i)
	end

	local link_index, link_count = obj.getpoint("link")
	link_index = link_index or 0
	link_count = link_count or 1

	local t = point_count <= 1 and 0.0 or clamp(index_value / (point_count - 1), 0.0, 1.0)
	local ok, timecontrol_value = pcall(obj.getpoint, "timecontrol", "value")
	if ok and timecontrol_value then
		t = timecontrol_value
	end

	return {
		values = values,
		linked_values = make_linked_values(point_count, link_index, link_count),
		axis_index = link_index + 1,
		t = t,
		segment = segment,
		local_t = local_t,
		edge_flags = {
			double_first = obj.getpoint("accelerate"),
			double_last = obj.getpoint("decelerate"),
		},
	}
end

function M.linear_move()
	local ctx = M.make_ctx()
	return linear_value(ctx, normalize_values(ctx.values or {}, ctx.divisor))
end

function M.linear_speed()
	local ctx = M.make_ctx()
	local axes = collect_axes(ctx)
	local segment, t = weighted_segment(ctx, axes)
	return linear_value({
		values = ctx.values,
		divisor = ctx.divisor,
		segment = segment,
		local_t = t,
		double_first = ctx.double_first,
		double_last = ctx.double_last,
		edge_flags = ctx.edge_flags,
	}, normalize_values(ctx.values or {}, ctx.divisor))
end

function M.linear_rotate()
	local ctx = M.make_ctx()
	local axes = rotation_axes(ctx)
	if axes then
		return linear_rotate_group_value(ctx, axes)
	end
	return rotation_linear_value(ctx, normalize_values(ctx.values or {}, ctx.divisor))
end

function M.interpolation_move()
	local ctx = M.make_ctx()
	local values = normalize_values(ctx.values or {}, ctx.divisor)
	local axes = collect_axes(ctx)
	local lengths = segment_lengths(axes, get_flags(ctx))
	return interpolation_value(ctx, values, lengths)
end

function M.interpolation_speed()
	local ctx = M.make_ctx()
	local axes = collect_axes(ctx)
	local segment, t, lengths = weighted_segment(ctx, axes)
	return interpolation_value({
		values = ctx.values,
		divisor = ctx.divisor,
		segment = segment,
		local_t = t,
		double_first = ctx.double_first,
		double_last = ctx.double_last,
		edge_flags = ctx.edge_flags,
	}, normalize_values(ctx.values or {}, ctx.divisor), lengths)
end

function M.interpolation_rotate()
	local ctx = M.make_ctx()
	local rotation_group = rotation_axes(ctx)
	if rotation_group then
		return interpolation_rotate_group_value(ctx, rotation_group)
	end

	local axes = collect_axes(ctx)
	local lengths = segment_lengths(axes, get_flags(ctx))
	return rotation_interpolation_value(ctx, normalize_values(ctx.values or {}, ctx.divisor), lengths)
end

function M.speed_move()
	local ctx = M.make_ctx()
	return speed_move_value(ctx, normalize_values(ctx.values or {}, ctx.divisor))
end

function M.random_move()
	local ctx = M.make_ctx()
	return random_move_value(ctx, normalize_values(ctx.values or {}, ctx.divisor))
end

function M.teleportation_move()
	local ctx = M.make_ctx()
	return teleport_value(ctx, normalize_values(ctx.values or {}, ctx.divisor))
end

return M
