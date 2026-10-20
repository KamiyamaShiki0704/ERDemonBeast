-- Real private/common HKS at the damage-dispatch seam. Resource IDs come from
-- the installed c0010 TAE; no Havok playback is simulated or claimed.
local common, original, animation_file = assert(arg[1]),assert(arg[2]),assert(arg[3])
local animations=dofile(animation_file)
local vars, events, damage_level, direction = {}, {}, 0, 2
function hkbGetVariable(name) return vars[name] or 0 end
function hkbIsNodeActive() return false end
function hkbFireEvent(name) events[#events+1]=name end
function act(id,name,value) if id==SetHavokVariable then vars[name]=value end end
function env(id,key)
    if id==GetHP or id==GetStamina then return 100 end
    if id==GetReceivedDamageDirection then return direction end
    if id==GetDamageLevel then return damage_level end
    if id==HasReceivedAnyDamage then return damage_level>0 and TRUE or FALSE end
    if id==GetAtkDirection then return ATTACK_DIR_FRONT end
    if id==DoesAnimExist then return animations[key] and TRUE or FALSE end
    if id==GetAIActionType or id==GetSpEffectEventAnimID then return -1 end
    if id==ObtainedDT then return 1000/60 end
    return 0
end
ERDB_TRACE_PATH=false; ERDB_BRIDGE_ENABLED=false
dofile(common); dofile(original); dofile(original)
local bases={W_DamageSmallFront=8030,W_DamageMiddleFront=8040,W_DamageLargeFront=8050,
    W_DamagePush=8120,W_DamageSmallBlow=8100,W_DamageLargeBlow=8101,W_DamageUpper=8130,
    W_DamageExtraLarge=8060,W_SABreak=8700}
for _,dir in ipairs({DIRECTION_FORWARD,DIRECTION_BACK,DIRECTION_LEFT,DIRECTION_RIGHT}) do
for level=1,11 do
    events={}
    damage_level,direction=level,dir
    ExecDamage(FALSE,FALSE)
    for _,event in ipairs(events) do
        local base=bases[event]
        assert(not base or animations[base], 'non-playable damage state dispatched: '..event..' requires '..tostring(base)..'; reference-pose/no cancel recovery risk')
    end
end
end
print('PASS: 44 direction/damage cases never select missing reaction clips')
