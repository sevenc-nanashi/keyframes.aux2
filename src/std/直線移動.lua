--speed:0,0

local num = obj.getpoint("num")
local index, ratio = math.modf(obj.getpoint("index"))
local st = obj.getpoint(index)
local ed = obj.getpoint(index + 1)

local accelerate = obj.getpoint("accelerate")
local decelerate = obj.getpoint("decelerate")

local function single_ease_in(t)
  return 1.5 * t * t - 0.5 * t * t * t
end

if accelerate and decelerate and num == 2 then
  if ratio < 0.5 then
    return st + (ed - st) * single_ease_in(ratio * 2) / 2
  else
    return st + (ed - st) * (1 - single_ease_in((1 - ratio) * 2) / 2)
  end
elseif accelerate and index == 0 then
  return st + (ed - st) * single_ease_in(ratio)
elseif decelerate and index == num - 2 then
  return st + (ed - st) * (1 - single_ease_in(1 - ratio))
else
  return st + (ed - st) * ratio
end
