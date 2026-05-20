--param:Bank ID (Do not edit manually),0
--param:Keyframe ID (Do not edit manually),0

local mod = obj.module("keyframes.aux2")
local bank_id, keyframe_id = obj.getpoint("param")
local index, _ratio = math.modf(obj.getpoint("index"))

local indices, script_name, script, accelerate, decelerate, params = mod.get_keyframe(bank_id, keyframe_id, index)

local inner_G = {}
local inner_obj = {}

inner_obj.getpoint = function(...)
  local args = { ... }
  local target = args[1]
  local option = args[2]
  local option2 = args[3]
  if type(target) == "number" then
    local remapped_index = indices[target + 1]
    if target < 0 then
      remapped_index = 0
    elseif target >= #indices then
      remapped_index = indices[#indices]
    end
    if #args > 1 then
      return obj.getpoint(remapped_index, option)
    else
      return obj.getpoint(remapped_index)
    end
  elseif target == "time" then
    if option then
      local remapped_index = indices[option + 1]
      if option < 0 then
        remapped_index = 0
      elseif option > #indices then
        remapped_index = indices[#indices]
      end
      return obj.getpoint("time", remapped_index) - obj.getpoint("time", indices[1])
    else
      return obj.getpoint("time") - obj.getpoint("time", indices[1])
    end
  elseif target == "accelerate" then
    return accelerate
  elseif target == "decelerate" then
    return decelerate
  elseif target == "index" then
    local current_time = obj.getpoint("time")
    for i = 1, #indices - 1 do
      local left_time = obj.getpoint("time", indices[i])
      local right_time = obj.getpoint("time", indices[i + 1])
      if current_time < left_time then
        return i - 1
      elseif current_time < right_time then
        return i - 1 + (current_time - left_time) / (right_time - left_time)
      end
    end
    return #indices - 1
  elseif target == "param" then
    return unpack(params)
  elseif target == "num" then
    return #indices
  else
    return obj.getpoint(unpack(args))
    -- local ret = { obj.getpoint(unpack(args)) }
    -- return unpack(ret)
  end
end
inner_G.obj = inner_obj

setmetatable(inner_obj, { __index = obj, __newindex = obj })
setmetatable(inner_G, { __index = _G, __newindex = _G })

SCRIPT_CACHE = SCRIPT_CACHE or {}
local f
if SCRIPT_CACHE[script_name] then
  f = SCRIPT_CACHE[script_name]
else
  local err
  f, err = loadstring(script, script_name)
  if not f then
    error("Failed to load keyframe script: " .. err)
  end
  SCRIPT_CACHE[script_name] = f
end

setfenv(f, inner_G)
local res = f()
return res
