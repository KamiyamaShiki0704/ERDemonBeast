-- Offline HKS seam only. Engine env/act are mocked; this is not a game test.
-- Usage: lua check_c0010_dispatch.lua <installed c9997.hks> <raw-r1|ai-3000>
local script, mode = assert(arg[1]), assert(arg[2])
assert(mode == "raw-r1" or mode == "ai-3000")
local events = {}
function hkbFireEvent(name) events[#events + 1] = name end
function hkbGetVariable() return 0 end
function hkbSetVariable() end
function act() end
function env(id, key)
    if id == GetAIActionType then return mode == "ai-3000" and 3000 or -1 end
    if id == ActionRequest then return key == ACTION_ARM_R1 and TRUE or FALSE end
    if id == DoesAnimExist then return TRUE end
    return 0
end
dofile(script)
local consumed = ExecAIAction(STYLE_DEFAULT)
local attack = false
for _, name in ipairs(events) do
    if name == "W_Attack3000" then attack = true end
end
print(string.format("mode=%s consumed=%s events=%s", mode, tostring(consumed), table.concat(events, ",")))
assert(attack, "R1 -> W_Attack3000 not dispatched at this HKS seam")
