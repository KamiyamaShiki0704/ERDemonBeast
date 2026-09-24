-- Real private/common/original HKS with graph-node events replayed at the seam.
-- Does not simulate Havok playback; graph trigger times are checked separately.
local common, original, dir, mode = arg[1], arg[2], arg[3], arg[4]
local node, ended, wait = nil, false, false
local blended_nodes = {}
local pressed, vars, events, calls = {}, {}, {}, {}
local missing = {[2001]=true,[2002]=true,[2003]=true}
ERDB_TRACE_PATH=false;ERDB_BRIDGE_DIR=dir
local function command(word, option)
    local f=assert(io.open(dir..'/behavior-command.txt','w'))
    f:write('1-2-3\n'..word..'\n'..(option or ''));f:close()
end
local function status()
    local f=assert(io.open(dir..'/behavior-status.txt','r'));local s=f:read('*a');f:close();return s
end
function act(id, key, value)
    calls[id]=(calls[id] or 0)+1
    if id==Wait then wait=true end
    if id==SetHavokVariable then vars[key]=value end
end
function hkbGetVariable(name) return vars[name] or 0 end
function hkbIsNodeActive(name) return name==node or blended_nodes[name]==true end
function hkbFireEvent(name)
    events[#events+1]=name
    if name=='W_Event20000' then node='Event20000_CMSG' end
    if name=='W_Event20010' then node='Event20010_CMSG' end
    if name=='W_Idle' then node='IdleDefault_CMSG' end
end
function env(id,key)
    if id==GetHP or id==GetStamina then return 100 end
    if id==GetAIActionType or id==GetSpEffectEventAnimID then return -1 end
    if id==GetFlyRouteState then return AI_FLY_ROUTE_STATE_NOT_USE_FLY_ROUTE end
    if id==DoesAnimExist then return missing[key] and FALSE or TRUE end
    if id==ObtainedDT then return 1000/60 end
    if id==IsAnimEnd then return ended and TRUE or FALSE end
    if id==ActionRequest then return wait and pressed[key] and TRUE or FALSE end
    if id==ActionCancelRequest then return pressed[key] and TRUE or FALSE end
    if id==ActionDuration then return pressed[key] and 10 or 0 end
    return 0
end
dofile(common);dofile(original);dofile(original)
local function tick(n) for _=1,n do wait=false;Update() end end
command('enter',mode=='fourway' and 'fourway\n' or '')
tick(1);IdleDefault_onActivate();IdleDefault_onUpdate()
ended=false;tick(1);ended=true;tick(1)
assert(status()=='1-2-3\nready\n','entry/option parsing regression')
ended=false;IdleDefault_onActivate()
if mode=='roll_cycles' then
    for cycle=1,20 do
        pressed[5]=true;events={};IdleDefault_onUpdate()
        assert(events[#events]=='W_Step6000','fresh roll rejected at cycle '..cycle)
        Step6000_onActivate();node='Step6000_CMSG';pressed={};tick(45)
        blended_nodes.IdleDefault_CMSG=true;IdleDefault_onActivate()
        pressed[5]=true;events={};IdleDefault_onUpdate()
        assert(#events==0,'already-active step redispatched during blend')
        node='IdleDefault_CMSG';blended_nodes={};IdleDefault_onActivate()
        -- The deferred input wasn't consumed or permanently marked held.
        IdleDefault_onUpdate()
        assert(events[#events]=='W_Step6000','valid roll blocked after blend ended')
        Step6000_onActivate();node='Step6000_CMSG';pressed={};tick(45)
        node='IdleDefault_CMSG';IdleDefault_onActivate();tick(1)
    end
    command('exit');tick(8);tick(1)
    assert(status()=='1-2-3\nfinishing\n','F7 stuck after repeated rolls')
    node='IdleDefault_CMSG';tick(1)
    assert(status()=='1-2-3\ndone\n','outro failed after repeated rolls')
elseif mode=='roll_blend' or mode=='roll_repeat' then
    pressed[5]=true;IdleDefault_onUpdate();Step6000_onActivate()
    node='Step6000_CMSG';pressed={};tick(45)
    if mode=='roll_blend' then
        -- Idle activates while Step is still blending out. A request for the
        -- already active Step can be ignored without a second onActivate.
        blended_nodes.IdleDefault_CMSG=true;IdleDefault_onActivate()
    end
    pressed[5]=true;events={}
    if mode=='roll_blend' then IdleDefault_onUpdate() else Step6000_onUpdate() end
    -- No synthetic activation: an ignored graph event must not freeze the
    -- adapter in pending_action forever after the real step has ended.
    node='IdleDefault_CMSG';blended_nodes={};IdleDefault_onActivate()
    pressed={};tick(1);pressed[0]=true;events={};IdleDefault_onUpdate()
    assert(events[#events]=='W_Attack3000','repeated roll left input permanently blocked after real step ended')
elseif mode=='outro_blend' then
    -- Havok reports BOTH outgoing Idle and incoming Event during blending.
    -- This is the v5 runtime trace: outro:active and done on the same frame.
    blended_nodes.IdleDefault_CMSG=true
    command('exit');tick(8)
    assert(status()=='1-2-3\nfinishing\n','outgoing Idle completed newly entered outro')
    tick(60)
    assert(status()=='1-2-3\nfinishing\n','elapsed frames completed active outro')
    blended_nodes.IdleDefault_CMSG=nil;tick(1)
    -- At the end, incoming Idle can also overlap the outgoing Event.
    blended_nodes.IdleBattle_CMSG=true;tick(1)
    assert(status()=='1-2-3\nfinishing\n','incoming Idle completed still-active outro')
    node=nil;blended_nodes={};tick(1)
    assert(status()=='1-2-3\nfinishing\n','no known return node acknowledged completion')
    node='IdleBattle_CMSG';tick(1)
    assert(status()=='1-2-3\ndone\n','completed outro transition never acknowledged')
elseif mode=='outro' then
    command('exit');tick(8);assert(status()=='1-2-3\nfinishing\n')
    tick(1);node='Attack3000_CMSG';tick(1)
    assert(status()=='1-2-3\nfinishing\n','foreign node acknowledged outro')
    node='Event20010_CMSG';tick(1)
    -- The 1011-converted clip trigger transitions to Idle; slot 1 stays false.
    node='IdleDefault_CMSG';tick(1)
    assert(status()=='1-2-3\ndone\n','outro stuck despite real graph completion event')
elseif mode=='skill' or mode=='skill_exit' then
    pressed[3]=true;IdleDefault_onUpdate();Attack3035_onActivate()
    node='Attack3035_CMSG';tick(2)
    if mode=='skill_exit' then command('exit');tick(8) end
    assert(status()=='1-2-3\nready\n','skill ended before its notification')
    node='Event20010_CMSG';tick(1)
    assert(status()=='1-2-3\ndone\n','skill 1011 event did not complete transformation')
else
    vars.IsLockon=1;vars.MoveSpeedLevel=0.5
    for _,angle in ipairs({0,90,-90,180}) do
        vars.MoveAngle=angle;events={};calls={};ExecMove(MOVE_TYPE_NONE,FALSE)
        if mode=='fourway' and angle~=0 then
            assert(#events==0,'missing lateral/back clip dispatched')
        elseif mode~='fourway' then
            assert(calls[LockonFixedAngleCancel],'original TDB heading unlock missing')
        end
    end
    if mode=='fourway' then
        missing={};vars.MoveAngle=90;events={};ExecMove(MOVE_TYPE_NONE,FALSE)
        assert(events[#events]=='W_WalkRightBattle','available four-way route not used')
        assert(not calls[LockonFixedAngleCancel],'four-way mode cancelled fixed facing')
    end
end
print('PASS v5 '..mode)
