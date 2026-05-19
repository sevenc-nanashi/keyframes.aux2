--speed:0,0

local DEG_TO_RAD = math.pi / 180
local RAD_TO_DEG = 180 / math.pi

local function matmul(a, b)
  return {
    {
      a[1][1] * b[1][1] + a[1][2] * b[2][1] + a[1][3] * b[3][1],
      a[1][1] * b[1][2] + a[1][2] * b[2][2] + a[1][3] * b[3][2],
      a[1][1] * b[1][3] + a[1][2] * b[2][3] + a[1][3] * b[3][3],
    },
    {
      a[2][1] * b[1][1] + a[2][2] * b[2][1] + a[2][3] * b[3][1],
      a[2][1] * b[1][2] + a[2][2] * b[2][2] + a[2][3] * b[3][2],
      a[2][1] * b[1][3] + a[2][2] * b[2][3] + a[2][3] * b[3][3],
    },
    {
      a[3][1] * b[1][1] + a[3][2] * b[2][1] + a[3][3] * b[3][1],
      a[3][1] * b[1][2] + a[3][2] * b[2][2] + a[3][3] * b[3][2],
      a[3][1] * b[1][3] + a[3][2] * b[2][3] + a[3][3] * b[3][3],
    },
  }
end

local function rotation_x(angle)
  local c = math.cos(angle)
  local s = math.sin(angle)
  return {
    { 1, 0, 0 },
    { 0, c, -s },
    { 0, s, c },
  }
end

local function rotation_y(angle)
  local c = math.cos(angle)
  local s = math.sin(angle)
  return {
    { c, 0, s },
    { 0, 1, 0 },
    { -s, 0, c },
  }
end

local function rotation_z(angle)
  local c = math.cos(angle)
  local s = math.sin(angle)
  return {
    { c, -s, 0 },
    { s, c, 0 },
    { 0, 0, 1 },
  }
end

local function matrix_from_xyz_degrees(x, y, z)
  return matmul(
    matmul(rotation_x(x * DEG_TO_RAD), rotation_y(y * DEG_TO_RAD)),
    rotation_z(z * DEG_TO_RAD)
  )
end

local function quaternion_from_matrix(m)
  local trace = m[1][1] + m[2][2] + m[3][3]

  if trace > 0 then
    local s = math.sqrt(trace + 1) * 2
    return {
      0.25 * s,
      (m[3][2] - m[2][3]) / s,
      (m[1][3] - m[3][1]) / s,
      (m[2][1] - m[1][2]) / s,
    }
  end

  if m[1][1] > m[2][2] and m[1][1] > m[3][3] then
    local s = math.sqrt(1 + m[1][1] - m[2][2] - m[3][3]) * 2
    return {
      (m[3][2] - m[2][3]) / s,
      0.25 * s,
      (m[1][2] + m[2][1]) / s,
      (m[1][3] + m[3][1]) / s,
    }
  end

  if m[2][2] > m[3][3] then
    local s = math.sqrt(1 + m[2][2] - m[1][1] - m[3][3]) * 2
    return {
      (m[1][3] - m[3][1]) / s,
      (m[1][2] + m[2][1]) / s,
      0.25 * s,
      (m[2][3] + m[3][2]) / s,
    }
  end

  local s = math.sqrt(1 + m[3][3] - m[1][1] - m[2][2]) * 2
  return {
    (m[2][1] - m[1][2]) / s,
    (m[1][3] + m[3][1]) / s,
    (m[2][3] + m[3][2]) / s,
    0.25 * s,
  }
end

local function normalize_quaternion(q)
  local length = math.sqrt(q[1] * q[1] + q[2] * q[2] + q[3] * q[3] + q[4] * q[4])
  assert(length > 0, "quaternion length must be positive")
  return { q[1] / length, q[2] / length, q[3] / length, q[4] / length }
end

local function slerp(a, b, alpha)
  local bw, bx, by, bz = b[1], b[2], b[3], b[4]
  local dot = a[1] * bw + a[2] * bx + a[3] * by + a[4] * bz

  if dot < 0 then
    bw, bx, by, bz = -bw, -bx, -by, -bz
    dot = -dot
  end

  if dot > 0.9995 then
    return normalize_quaternion({
      a[1] + (bw - a[1]) * alpha,
      a[2] + (bx - a[2]) * alpha,
      a[3] + (by - a[3]) * alpha,
      a[4] + (bz - a[4]) * alpha,
    })
  end

  local theta = math.acos(dot)
  local sin_theta = math.sin(theta)
  assert(sin_theta ~= 0, "slerp angle must not be singular")

  local scale_a = math.sin((1 - alpha) * theta) / sin_theta
  local scale_b = math.sin(alpha * theta) / sin_theta

  return {
    a[1] * scale_a + bw * scale_b,
    a[2] * scale_a + bx * scale_b,
    a[3] * scale_a + by * scale_b,
    a[4] * scale_a + bz * scale_b,
  }
end

local function matrix_from_quaternion(q)
  local w, x, y, z = q[1], q[2], q[3], q[4]
  return {
    { 1 - 2 * y * y - 2 * z * z, 2 * x * y - 2 * z * w, 2 * x * z + 2 * y * w },
    { 2 * x * y + 2 * z * w, 1 - 2 * x * x - 2 * z * z, 2 * y * z - 2 * x * w },
    { 2 * x * z - 2 * y * w, 2 * y * z + 2 * x * w, 1 - 2 * x * x - 2 * y * y },
  }
end

local function clamp_unit(value)
  if value < -1 then
    return -1
  end
  if value > 1 then
    return 1
  end
  return value
end

local function xyz_degrees_from_matrix(m)
  local y = math.asin(clamp_unit(m[1][3]))
  local x = math.atan2(-m[2][3], m[3][3])
  local z = math.atan2(-m[1][2], m[1][1])
  return x * RAD_TO_DEG, y * RAD_TO_DEG, z * RAD_TO_DEG
end

local index, ratio = math.modf(obj.getpoint("index"))
local link_index = obj.getpoint("link")

local st_x = obj.getpoint(index, 0 - link_index)
local st_y = obj.getpoint(index, 1 - link_index)
local st_z = obj.getpoint(index, 2 - link_index)
local ed_x = obj.getpoint(index + 1, 0 - link_index)
local ed_y = obj.getpoint(index + 1, 1 - link_index)
local ed_z = obj.getpoint(index + 1, 2 - link_index)

local from = { st_x, st_y, st_z }
local to = { ed_x, ed_y, ed_z }
local from_quaternion = quaternion_from_matrix(matrix_from_xyz_degrees(from[1], from[2], from[3]))
local to_quaternion = quaternion_from_matrix(matrix_from_xyz_degrees(to[1], to[2], to[3]))
local interpolated = slerp(from_quaternion, to_quaternion, ratio)
local rx, ry, rz = xyz_degrees_from_matrix(matrix_from_quaternion(interpolated))

if link_index == 0 then
  return rx
elseif link_index == 1 then
  return ry
elseif link_index == 2 then
  return rz
end
