-- Actual HKS regression seam; adversarial stale cancel signal during blending.
local common, original, mode = arg[1], arg[2], arg[3]
local pressed, duration, events, vars = {}, {}, {}, {}
local wait, cancel, ended = false, true, false
ERDB_TRACE_PATH = false
ERDB_BRIDGE_ENABLED = false
if mode=='lifecycle' then
    ERDB_BRIDGE_ENABLED=true
    ERDB_BRIDGE_DIR=assert(arg[4])
    local f=assert(io.open(ERDB_BRIDGE_DIR..'/behavior-command.txt','w'));f:write('1-2-3\nenter\n');f:close()
end
function act(id, name, value)
    if id == Wait then wait = true end
    if id == SetHavokVariable then vars[name] = value end
end
function hkbGetVariable(name) return vars[name] or 0 end
function hkbFireEvent(name) events[#events+1] = name end
function hkbIsNodeActive() return false end
function env(id, key)
    if id == GetHP or id == GetStamina then return 100 end
    if id == GetReceivedDamageType then return DAMAGE_TYPE_NONE end
    if id == GetAIActionType or id == GetSpEffectEventAnimID then return -1 end
    if id == GetFlyRouteState then return AI_FLY_ROUTE_STATE_NOT_USE_FLY_ROUTE end
    if id == DoesAnimExist then return TRUE end
    if id == ObtainedDT then return 1000/60 end
    if id == IsAnimEnd then return ended and TRUE or FALSE end
    if id == ActionRequest then return wait and pressed[key] and TRUE or FALSE end
    if id == ActionCancelRequest then return cancel and pressed[key] and TRUE or FALSE end
    if id == ActionDuration then return duration[key] or 0 end
    return 0
end
dofile(common); dofile(original); dofile(original)
local function tick(n)
    for _=1,n do wait=false; Update() end
end
if mode == 'lifecycle' then
    local function status()
        local f=assert(io.open(ERDB_BRIDGE_DIR..'/behavior-status.txt','r'));local s=f:read('*a');f:close();return s
    end
    tick(1);assert(events[#events]=='W_Event20000','entry animation was not requested')
    ended=true;Event20000_onUpdate();assert(status()=='1-2-3\nstarting\n','stale end accepted before activation')
    Event20000_onActivate();ended=false;Event20000_onUpdate();assert(status()=='1-2-3\nstarting\n')
    ended=true;Event20000_onUpdate();assert(status()=='1-2-3\nready\n')
    ended=false;IdleDefault_onActivate();pressed[0]=true;duration[0]=10
    IdleDefault_onUpdate();Attack3000_onActivate()
    local f=assert(io.open(ERDB_BRIDGE_DIR..'/behavior-command.txt','w'));f:write('1-2-3\nexit\n');f:close()
    events={};tick(8);Attack3000_onUpdate()
    assert(#events==0,'expiry/manual request interrupted unfinished attack')
    ended=true;Attack3000_onUpdate();assert(events[#events]=='W_Event20010')
    assert(status()=='1-2-3\nfinishing\n')
    Event20010_onUpdate();assert(status()=='1-2-3\nfinishing\n','stale end accepted before activation')
    Event20010_onActivate();ended=false;Event20010_onUpdate();assert(status()=='1-2-3\nfinishing\n')
    ended=true;Event20010_onUpdate();assert(status()=='1-2-3\ndone\n')
elseif mode == 'movement' then
    vars.IsLockon=1;vars.MoveSpeedLevel=0.5
    for _,v in ipairs({{-90,'W_WalkLeftBattle'},{90,'W_WalkRightBattle'},{180,'W_WalkBackBattle'}}) do
        events={};vars.MoveAngle=v[1]
        ExecMove(MOVE_TYPE_NONE,FALSE)
        assert(events[#events]==v[2], 'locked direction collapsed: expected '..v[2]..', got '..tostring(events[#events]))
    end
else
    tick(1);pressed[0]=true;duration[0]=10
    IdleDefault_onUpdate();assert(events[#events]=='W_Attack3000')
    local count=#events;IdleDefault_onUpdate();assert(#events==count,'dispatch/activation gap allowed restart')
    Attack3000_onActivate()
    pressed={};duration={};tick(2)
    pressed[0]=true;duration[0]=10;events={}
    Attack3000_onUpdate()
    assert(#events==0, 'stale global cancel interrupted attack before its TAE window')
    -- A late outgoing Idle callback cannot treat this as a fresh idle attack.
    IdleDefault_onUpdate()
    assert(#events==0, 'outgoing idle callback bypassed current action owner')
    tick(47);Attack3000_onUpdate()
    assert(events[#events]=='W_Attack3001','combo did not open at its own TAE window')
end
print('PASS '..mode)
