--param:Bank ID (Do not edit manually),0
--param:Keyframe ID (Do not edit manually),0

local mod = obj.module("keyframes.aux2")
local bank_id, keyframe_id = obj.getpoint("param")
local index, _ratio = math.modf(obj.getpoint("index"))

local starts_at, ends_at, script_name, script, accelerate, decelerate = mod.get_keyframe(bank_id, keyframe_id, index)

local inner_G = {}
local inner_obj = {}

inner_obj.getpoint = function(...)
  local args = { ... }
  local target = args[1]
  local option = args[2]
  local option2 = args[3]
  if type(target) == "number" then
    if #args > 1 then
      return obj.getpoint(target + starts_at, option)
    else
      return obj.getpoint(target + starts_at)
    end
  elseif target == "time" then
    if option then
      return obj.getpoint("time", option + starts_at) - obj.getpoint("time", starts_at)
    else
      return obj.getpoint("time") - obj.getpoint("time", starts_at)
    end
  elseif target == "accelerate" then
    return accelerate
  elseif target == "decelerate" then
    return decelerate
  elseif target == "index" then
    return obj.getpoint("index") - starts_at
  elseif target == "param" then
    return 0
  elseif target == "num" then
    return ends_at - starts_at + 2
  else
    return obj.getpoint(unpack(args))
    -- local ret = { obj.getpoint(unpack(args)) }
    -- return unpack(ret)
  end
end
inner_G.obj = inner_obj

setmetatable(inner_obj, { __index = obj, __newindex = obj })
setmetatable(inner_G, { __index = _G, __newindex = _G })

local f, err = loadstring(script, script_name)
if not f then
  error("Failed to load keyframe script: " .. err)
end

setfenv(f, inner_G)
local success, result = pcall(f)
if not success then
  error("Error executing keyframe script: " .. result)
end

return result
