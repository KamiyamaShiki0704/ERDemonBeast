-- Offline real-HKS seam. Engine conditions mocked; no gameplay PASS implied.
-- args: common or generated private HKS, original decompiled c0010, adapter or baseline or bundled
local common, original, adapter = assert(arg[1]), assert(arg[2]), assert(arg[3])
local input, timers, events, vars = {}, {}, {}, {}
local stamina, cancel, missing, ai, damage = 100, false, nil, -1, false
local animations = {[0]=true,[3000]=true,[3001]=true,[3002]=true,[3003]=true,[3005]=true,
    [3007]=true,[3010]=true,[3030]=true,[3035]=true,[6000]=true}
function hkbFireEvent(name) events[#events+1] = name end
function hkbGetVariable(name) return vars[name] or 0 end
function hkbIsNodeActive() return false end
function act(id, name, value)
    if id == SetHavokVariable then vars[name] = value end
end
function env(id, key)
    if id == GetHP then return 100 end
    if id == GetStamina then return stamina end
    if id == GetReceivedDamageType then return damage and DAMAGE_TYPE_DEATHIDLE or DAMAGE_TYPE_NONE end
    if id == GetAIActionType then return ai end
    if id == GetSpEffectEventAnimID then return -1 end
    if id == ActionRequest then return input[key] and TRUE or FALSE end
    if id == ActionCancelRequest then return cancel and input[key] and TRUE or FALSE end
    if id == ActionDuration then return timers[key] or 0 end
    if id == DoesAnimExist then return animations[key] and key ~= missing and TRUE or FALSE end
    return 0
end
dofile(common)
if adapter ~= 'baseline' and adapter ~= 'bundled' then dofile(adapter) end
-- The behavior's luaFiles lists the original c0010 twice. Reproduce that ordering.
dofile(original)
dofile(original)
local function release()
    input, timers, events = {}, {}, {}
    Update()
    events = {}
end
local function press(key)
    input[key], timers[key] = true, 0.1
end
local function expect(name)
    assert(events[#events] == name, 'expected '..name..', got '..tostring(events[#events]))
end
press(0)
IdleDefault_onUpdate()
expect('W_Attack3000') -- FAIL on the actual common script without the adapter.
if adapter == 'baseline' then error('baseline unexpectedly passes') end
for _, route in ipairs({{0,3000},{1,3005},{2,3030},{3,3035},{5,6000}}) do
    release(); press(route[1]); IdleDefault_onUpdate()
    expect((route[2] == 6000 and 'W_Step' or 'W_Attack') .. route[2])
    local count = #events
    IdleDefault_onUpdate()
    assert(#events == count, 'held input restarted an action')
end
for _, route in ipairs({{3000,0,3001},{3001,0,3002},{3002,0,3003},{3003,0,3000},
    {3005,0,3001},{3005,1,3007},{3007,0,3000},{6000,0,3010},{3010,1,3007}}) do
    release(); press(route[2]); cancel = false
    _G[(route[1] == 6000 and 'Step' or 'Attack')..route[1]..'_onUpdate']()
    assert(#events == 0, 'closed cancel window was bypassed')
    cancel = true
    _G[(route[1] == 6000 and 'Step' or 'Attack')..route[1]..'_onUpdate']()
    expect('W_Attack'..route[3])
end
release(); press(0); stamina = 0; vars.StaminaComboResetTest = 1
IdleDefault_onUpdate(); assert(#events == 0, 'exhaustion ignored')
release(); press(0); stamina = 100; cancel = true
Attack3001_onUpdate(); expect('W_Attack3000')
release(); press(0); missing = 3000
IdleDefault_onUpdate(); assert(#events == 0, 'missing animation dispatched')
missing = nil
release(); ai = 3000
IdleDefault_onUpdate(); expect('W_Attack3000'); ai = -1
release(); press(0); damage = true
Attack3000_onUpdate(); expect('W_DeathIdleDefault'); damage = false
release(); press(0); cancel = true
Event20000_onUpdate(); expect('W_Attack3000')
release(); press(17)
IdleDefault_onUpdate(); assert(#events == 0, 'ER rolling became NR ULTRA')
print('PASS: original c0010 + ER common/private HKS; idle, five actions, nine combo edges, cancel gates, hold/release, stamina reset, missing animation, AI fallback, death priority, original Event20000, NR/ER index collision')
