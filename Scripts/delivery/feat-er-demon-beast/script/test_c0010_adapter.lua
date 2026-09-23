-- Offline real-HKS seam. Engine conditions mocked; no gameplay PASS implied.
-- args: common or generated private HKS, original decompiled c0010, adapter or baseline or bundled
local common, original, adapter = assert(arg[1]), assert(arg[2]), assert(arg[3])
local input, timers, events, vars = {}, {}, {}, {}
local stamina, cancel, missing, ai, damage = 100, false, nil, -1, false
-- ER 2.7.1 env(1106): Wait selects new presses; otherwise it reads
-- queued inputs. The old mock incorrectly returned raw presses unconditionally.
local waiting, queued = false, {}
local animations = {[0]=true,[3000]=true,[3001]=true,[3002]=true,[3003]=true,[3005]=true,
    [3007]=true,[3010]=true,[3030]=true,[3035]=true,[6000]=true}
function hkbFireEvent(name)
    events[#events+1] = name
    local callback=_G[name:gsub('^W_','')..'_onActivate']
    if callback then callback() end
end
function hkbGetVariable(name) return vars[name] or 0 end
function hkbIsNodeActive() return false end
function act(id, name, value)
    if id == SetHavokVariable then vars[name] = value end
    if id == Wait then waiting = true end
end
function env(id, key)
    if id == ObtainedDT then return 1000/60 end
    if id == GetFlyRouteState then return -1 end
    if id == GetHP then return 100 end
    if id == GetStamina then return stamina end
    if id == GetReceivedDamageType then return damage and DAMAGE_TYPE_DEATHIDLE or DAMAGE_TYPE_NONE end
    if id == GetAIActionType then return ai end
    if id == GetSpEffectEventAnimID then return -1 end
    if id == ActionRequest then
        if waiting then return input[key] and TRUE or FALSE end
        return queued[key] and TRUE or FALSE
    end
    if id == ActionCancelRequest then return cancel and input[key] and TRUE or FALSE end
    if id == ActionDuration then return timers[key] or 0 end
    if id == DoesAnimExist then return animations[key] and key ~= missing and TRUE or FALSE end
    return 0
end
ERDB_TRACE_PATH = false -- Offline tests must not write a runtime log.
ERDB_BRIDGE_ENABLED = false
if arg[4] then
    local file = assert(io.open(common, 'rb'))
    local contents = file:read('*a'); file:close()
    -- Match Lua loadfile's UTF-8 BOM handling before overriding chunk name.
    if contents:sub(1,3) == '\239\187\191' then contents = contents:sub(4) end
    assert(load(contents, arg[4]))()
else dofile(common) end
if adapter ~= 'baseline' and adapter ~= 'bundled' then dofile(adapter) end
-- The behavior's luaFiles lists the original c0010 twice. Reproduce that ordering.
dofile(original)
dofile(original)
local function activate(id)
    _G[(id==6000 and 'Step' or 'Attack')..id..'_onActivate']()
end
local function tick(n) for _=1,n do Update() end end
local function release()
    IdleDefault_onActivate()
    input, timers, events = {}, {}, {}
    waiting, queued = false, {}
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
for _, callback in ipairs({RunDefault_onUpdate, WalkFrontDefault_onUpdate}) do
    release(); press(0); callback(); expect('W_Attack3000')
    release(); press(5); callback(); expect('W_Step6000')
end
for _, route in ipairs({{3000,0,3001},{3001,0,3002},{3002,0,3003},{3003,0,3000},
    {3005,0,3001},{3005,1,3007},{3007,0,3000},{6000,0,3010},{3010,1,3007}}) do
    release(); activate(route[1]); tick(150); press(route[2]); cancel = false
    _G[(route[1] == 6000 and 'Step' or 'Attack')..route[1]..'_onUpdate']()
    assert(#events == 0, 'closed cancel window was bypassed')
    cancel = true
    _G[(route[1] == 6000 and 'Step' or 'Attack')..route[1]..'_onUpdate']()
    expect('W_Attack'..route[3])
end
release(); press(0); stamina = 0; vars.StaminaComboResetTest = 1
IdleDefault_onUpdate(); assert(#events == 0, 'exhaustion ignored')
release(); activate(3001); tick(150); press(0); stamina = 100; cancel = true
Attack3001_onUpdate(); expect('W_Attack3000')
release(); press(0); missing = 3000
IdleDefault_onUpdate(); assert(#events == 0, 'missing animation dispatched')
missing = nil
release(); ai = 3000
IdleDefault_onUpdate(); expect('W_Attack3000'); ai = -1
release(); activate(3000); press(0); damage = true
Attack3000_onUpdate(); expect('W_DeathIdleDefault'); damage = false
release(); press(0); cancel = true
Event20000_onUpdate(); assert(#events==0, 'intro without an owned action bypassed window')
release(); press(17)
IdleDefault_onUpdate(); assert(#events == 0, 'ER rolling became NR ULTRA')
-- Optional diagnostic IO failures cannot suppress gameplay.
local saved_io = io
ERDB_TRACE_PATH = 'offline-no-file'
io = {open=function() error('simulated log IO failure') end}
release(); press(0); IdleDefault_onUpdate(); expect('W_Attack3000')
io = saved_io; ERDB_TRACE_PATH = false
print('PASS: idle/walk/run from new presses without queued inputs; five actions; nine combo edges; cancel gates; hold/release; stamina; missing animation; AI fallback; death; Event20000; NR/ER index collision; diagnostic IO failure')
