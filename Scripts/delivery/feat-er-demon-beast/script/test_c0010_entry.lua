-- Startup ordering seam: a root Update precedes the initial state's activation.
-- No synthetic Event onActivate callback: observe the real node and end flag.
local private, original, root = assert(arg[1]), assert(arg[2]), assert(arg[3])
ERDB_TRACE_PATH=false;ERDB_BRIDGE_ENABLED=true;ERDB_BRIDGE_DIR=root
local live=false
local node=nil
local ended=false
local fired={}
local vars={}
function hkbGetVariable(name) return vars[name] or 0 end
function hkbIsNodeActive(name) return name==node end
function hkbFireEvent(name)
    if not live then return end -- Root graph has not entered its initial state yet.
    fired[#fired+1]=name
    if name=='W_Event20000' then node='Event20000_CMSG';ended=false end
    if name=='W_Event20010' then node='Event20010_CMSG';ended=false end
    if name=='W_Idle' then node='IdleDefault_CMSG';ended=false;IdleDefault_onActivate() end
end
function act(id,name,value) if id==SetHavokVariable then vars[name]=value end end
function env(id,key)
    if id==GetHP or id==GetStamina then return 100 end
    if id==GetReceivedDamageType then return DAMAGE_TYPE_NONE end
    if id==GetAIActionType or id==GetSpEffectEventAnimID or id==GetFlyRouteState then return -1 end
    if id==DoesAnimExist then return TRUE end
    if id==ObtainedDT then return 1000/60 end
    if id==IsAnimEnd then return ended and TRUE or FALSE end
    return 0
end
local function command(text)
    local f=assert(io.open(root..'/behavior-command.txt','w'));f:write('4-5-6\n'..text..'\n');f:close()
end
local function status()
    local f=io.open(root..'/behavior-status.txt','r');if not f then return '' end
    local s=f:read('*a');f:close();return s
end
command('enter');os.remove(root..'/behavior-status.txt')
dofile(private);dofile(original);dofile(original)
Update() -- During bootstrap, before Idle activates.
live=true;node='IdleDefault_CMSG';IdleDefault_onActivate()
for _=1,5 do Update();IdleDefault_onUpdate() end
assert(node=='Event20000_CMSG','entry event was lost before initial state could receive it')
assert(status()=='4-5-6\nstarting\n','entry did not start')
-- A stale finished bit on an unrelated node must not confirm entry.
node='IdleDefault_CMSG';ended=true;Update()
assert(status()~='4-5-6\nready\n','unrelated node acknowledged entry')
node='Event20000_CMSG';ended=false;Update();Event20000_onUpdate()
assert(status()=='4-5-6\nstarting\n')
ended=true;Update();assert(status()=='4-5-6\nready\n','actual intro end never acknowledged')
assert(node=='IdleDefault_CMSG')
command('exit')
for _=1,8 do Update();IdleDefault_onUpdate() end
assert(node=='Event20010_CMSG','outro did not start')
ended=false;Update();assert(status()=='4-5-6\nfinishing\n')
ended=true;Update();assert(status()=='4-5-6\nfinishing\n','outro accepted slot instead of graph trigger')
node='IdleDefault_CMSG';Update();assert(status()=='4-5-6\ndone\n','actual outro graph event never acknowledged')
print('PASS: bootstrap ordering, no synthetic event activation, actual node/end, stale unrelated end, entry/exit acknowledgements')
