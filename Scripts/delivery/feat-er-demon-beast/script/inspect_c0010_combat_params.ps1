param([Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference='Stop'
$deps='D:\_Ugit\FSTool\FSModify\UnpackingTools\AquaToolset\Aqua.Toolset_2026.7.1\net9.0-windows7.0'
$paramdex='D:\_Ugit\FSTool\FSModify\UnpackingTools\WitchyBND\WitchyBND-3.0.1.0-win-x64\Assets\Paramdex'
Add-Type -Path (Join-Path $deps 'BouncyCastle.Cryptography.dll')
Add-Type -Path (Join-Path $deps 'SoulsFormats.dll')
[void][Runtime.InteropServices.NativeLibrary]::Load('F:\SteamLibrary\steamapps\common\ELDEN RING\Game\oo2core_6_win64.dll')
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
function Values($row) {
    $values=[ordered]@{}
    foreach($cell in $row.Cells) {
        if($cell.Def.DisplayType -ne 'dummy8') {$values[$cell.Def.InternalName]=$cell.Value}
    }
    @{id=$row.ID;name=$row.Name;fields=$values}
}
$gameFiles=@{ER='F:\GoldenAge\_GoldenAge\GA\regulation.bin';NR='F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\regulation.bin'}
$gameParams=@{}
$schemas=@()
foreach($game in @('ER','NR')) {
    $bnd=if($game -eq 'NR') {[SoulsFormats.Cryptography.RegulationDecryptor]::DecryptERNRRegulation($gameFiles[$game])} else {[SoulsFormats.Cryptography.RegulationDecryptor]::DecryptERRegulation($gameFiles[$game])}
    $params=@{}
    $definitions=@{}
    $defRoot=if($game -eq 'NR') {'D:\_Ugit\_FTProject\SmithBox_FT\Smithbox_FT_Temp_260508\03-Smithbox-Binary\Assets\PARAM\NR\Defs'} else {Join-Path $paramdex 'ER/Defs'}
    foreach($defFile in Get-ChildItem $defRoot -Filter '*.xml') {
        [xml]$xml=Get-Content -LiteralPath $defFile.FullName -Raw
        $definitions[[string]$xml.PARAMDEF.ParamType]=$defFile.FullName
    }
    foreach($name in @('NpcParam','NpcThinkParam','ChrModelParam','BehaviorParam','AtkParam_Npc','Bullet','SpEffectParam','SpEffectVfxParam')) {
        $entry=@($bnd.Files | Where-Object {[IO.Path]::GetFileNameWithoutExtension($_.Name) -eq $name})
        if($entry.Count -ne 1) {throw "Unexpected table count: $game/$name"}
        $param=[SoulsFormats.PARAM]::Read([byte[]]$entry[0].Bytes)
        $defPath=$definitions[$param.ParamType]
        if(-not $defPath) {throw "No definition: $game/$name/$($param.ParamType)"}
        $def=[SoulsFormats.PARAMDEF]::XmlDeserialize($defPath,$false,$false)
        $schemas+=@{game=$game;table=$name;data_version=$param.ParamdefDataVersion;def_version=$def.DataVersion;row_size=$param.DetectedSize;def_size=$def.GetRowSize(0);rows=$param.Rows.Count}
        if($param.ParamType -ne $def.ParamType -or $param.DetectedSize -ne $def.GetRowSize(0)) {throw "Schema size/type mismatch: $game/$name actual=$($param.DetectedSize) def=$($def.GetRowSize(0)) version=$($param.ParamdefDataVersion)/$($def.DataVersion)"}
        $param.ApplyParamdef($def)
        $params[$name]=$param
    }
    $gameParams[$game]=$params
}
$schemas | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $OutputDirectory 'schemas.json') -Encoding utf8
$nr=$gameParams.NR
$rows=@{}
$rows.NpcParam=@(Values $nr.NpcParam[100000])
$rows.NpcThinkParam=@(Values $nr.NpcThinkParam[100000])
$rows.ChrModelParam=@(Values $nr.ChrModelParam[100])
$rows.BehaviorParam=@($nr.BehaviorParam.Rows | Where-Object {$_['variationId'].Value -eq 100} | ForEach-Object {Values $_})
$atk=[Collections.Generic.HashSet[int]]::new()
$bullets=[Collections.Generic.HashSet[int]]::new()
$effects=[Collections.Generic.HashSet[int]]::new()
foreach($r in $rows.BehaviorParam) {
    if($r.fields.refType -eq 0) {[void]$atk.Add($r.fields.refId)}
    elseif($r.fields.refType -eq 1) {[void]$bullets.Add($r.fields.refId)}
    elseif($r.fields.refType -eq 2) {[void]$effects.Add($r.fields.refId)}
}
foreach($cell in $nr.NpcParam[100000].Cells) {
    if($cell.Def.InternalName -match '^spEffectID\d+$' -and $cell.Value -ge 0) {[void]$effects.Add($cell.Value)}
}
$rows.AtkParam_Npc=@(foreach($id in $atk) {if($nr.AtkParam_Npc[$id]) {Values $nr.AtkParam_Npc[$id]}})
$rows.Bullet=@(foreach($id in $bullets) {if($nr.Bullet[$id]) {Values $nr.Bullet[$id]}})
$rows.SpEffectParam=@(foreach($id in $effects) {if($nr.SpEffectParam[$id]) {Values $nr.SpEffectParam[$id]}})
# Traverse combat links separately from shared NPC effects; preserve existing ER
# dependencies instead of pulling unrelated NR systems through common IDs.
$bulletQueue=[Collections.Generic.Queue[int]]::new()
foreach($id in $bullets) {$bulletQueue.Enqueue($id)}
while($bulletQueue.Count) {
    $row=$nr.Bullet[$bulletQueue.Dequeue()]
    if(-not $row) {continue}
    if($row['atkId_Bullet'].Value -ge 0) {[void]$atk.Add($row['atkId_Bullet'].Value)}
    foreach($field in @('HitBulletID','intervalCreateBulletId')) {
        $id=[int]$row[$field].Value
        if($id -ge 0 -and $bullets.Add($id)) {$bulletQueue.Enqueue($id)}
    }
    foreach($field in @('spEffectIDForShooter','spEffectId0','spEffectId1','spEffectId2','spEffectId3','spEffectId4')) {
        if($row[$field].Value -gt 0) {[void]$effects.Add($row[$field].Value)}
    }
}
foreach($id in $atk) {
    $row=$nr.AtkParam_Npc[$id]
    if($row) {foreach($field in @('spEffectId0','spEffectId1','spEffectId2','spEffectId3','spEffectId4')) {
        if($row[$field].Value -gt 0) {[void]$effects.Add($row[$field].Value)}
    }}
}
$rows.AtkParam_Npc=@(foreach($id in $atk) {if($nr.AtkParam_Npc[$id]) {Values $nr.AtkParam_Npc[$id]}})
$rows.Bullet=@(foreach($id in $bullets) {if($nr.Bullet[$id]) {Values $nr.Bullet[$id]}})
$rows.SpEffectParam=@(foreach($id in $effects) {if($nr.SpEffectParam[$id]) {Values $nr.SpEffectParam[$id]}})
$rows | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $OutputDirectory 'nr-direct-rows.json') -Encoding utf8
$collisions=@()
foreach($name in $rows.Keys) {
    foreach($row in $rows[$name]) {
        $existing=$gameParams.ER[$name][$row.id]
        $collisions+=@{table=$name;id=$row.id;nr_name=$row.name;er_present=($null -ne $existing);er_row=if($existing){Values $existing}else{$null}}
    }
}
$collisions | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $OutputDirectory 'er-collisions.json') -Encoding utf8
$ani=[SoulsFormats.BND4]::Read('F:\GoldenAge\_GoldenAge\GA\chr\c0010.anibnd.dcx')
$tae=[SoulsFormats.TAE]::Read([byte[]]@($ani.Files | Where-Object Name -like '*.tae')[0].Bytes)
$events=@(foreach($a in $tae.Animations) {foreach($e in $a.Events) {
    $bytes=$e.GetParameterBytes($false)
    $ints=@(for($i=0;$i+4 -le $bytes.Length;$i+=4) {[BitConverter]::ToInt32($bytes,$i)})
    @{animation=$a.ID;type=$e.Type;start=$e.StartTime;end=$e.EndTime;raw=[Convert]::ToHexString($bytes);ints=$ints}
}})
$events | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $OutputDirectory 'tae-events.json') -Encoding utf8
foreach($e in $events) {if($e.type -in @(66,67,401)) {[void]$effects.Add($e.ints[0])}}
$effectQueue=[Collections.Generic.Queue[int]]::new()
foreach($id in $effects) {$effectQueue.Enqueue($id)}
while($effectQueue.Count) {
    $id=$effectQueue.Dequeue()
    $row=$nr.SpEffectParam[$id]
    if(-not $row -or $gameParams.ER.SpEffectParam[$id]) {continue}
    foreach($field in @('replaceSpEffectId','cycleOccurrenceSpEffectId','atkOccurrenceSpEffectId')) {
        $value=[int]$row[$field].Value
        if($value -gt 0 -and $effects.Add($value)) {$effectQueue.Enqueue($value)}
    }
}
$rows.SpEffectParam=@(foreach($id in $effects) {if($nr.SpEffectParam[$id]) {Values $nr.SpEffectParam[$id]}})
$rows | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $OutputDirectory 'nr-closure-rows.json') -Encoding utf8
$vfxIds=@($rows.SpEffectParam | Where-Object {$_.fields.vfxId -gt 0 -and -not $gameParams.ER.SpEffectParam[$_.id]} | ForEach-Object {$_.fields.vfxId} | Sort-Object -Unique)
$vfxRows=@(foreach($id in $vfxIds) {
    @{id=$id;nr=if($nr.SpEffectVfxParam[$id]){Values $nr.SpEffectVfxParam[$id]}else{$null};er=if($gameParams.ER.SpEffectVfxParam[$id]){Values $gameParams.ER.SpEffectVfxParam[$id]}else{$null}}
})
$vfxRows | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $OutputDirectory 'vfx-dependencies.json') -Encoding utf8
$collisions=@()
foreach($name in $rows.Keys) {foreach($row in $rows[$name]) {
    $existing=$gameParams.ER[$name][$row.id]
    $collisions+=@{table=$name;id=$row.id;nr_name=$row.name;er_present=($null -ne $existing);er_row=if($existing){Values $existing}else{$null}}
}}
$collisions | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $OutputDirectory 'er-closure-collisions.json') -Encoding utf8
[ordered]@{row_counts=@($rows.Keys | Sort-Object | ForEach-Object {@{table=$_;rows=$rows[$_].Count;ids=@($rows[$_].id)}});tae_types=@($events | Group-Object type | Sort-Object {[int]$_.Name} | ForEach-Object {@{type=[int]$_.Name;count=$_.Count}})} | ConvertTo-Json -Depth 6
