param(
    [string]$Regulation='F:\GoldenAge\_GoldenAge\GA\regulation.bin',
    [string]$Nightreign='F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\regulation.bin',
    [string]$Animations='F:\GoldenAge\_GoldenAge\GA\chr\c0010.anibnd.dcx',
    [Parameter(Mandatory=$true)][string]$OutputDirectory
)
$ErrorActionPreference='Stop'
$deps='D:\_Ugit\FSTool\FSModify\UnpackingTools\AquaToolset\Aqua.Toolset_2026.7.1\net9.0-windows7.0'
$erDefs='D:\_Ugit\FSTool\FSModify\UnpackingTools\WitchyBND\WitchyBND-3.0.1.0-win-x64\Assets\Paramdex\ER\Defs'
$nrDefs='D:\_Ugit\_FTProject\SmithBox_FT\Smithbox_FT_Temp_260508\03-Smithbox-Binary\Assets\PARAM\NR\Defs'
Add-Type -Path (Join-Path $deps 'BouncyCastle.Cryptography.dll')
Add-Type -Path (Join-Path $deps 'SoulsFormats.dll')
[void][Runtime.InteropServices.NativeLibrary]::Load('F:\SteamLibrary\steamapps\common\ELDEN RING\Game\oo2core_6_win64.dll')
function Hash([string]$path) {(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()}
function Digest([byte[]]$bytes) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()}
function Entry($bnd,[string]$name) {
    $files=@($bnd.Files | Where-Object {[IO.Path]::GetFileNameWithoutExtension($_.Name) -eq $name})
    if($files.Count -ne 1) {throw "Expected one table: $name"}
    $files[0]
}
$offset=[SoulsFormats.PARAM+Row].GetField('DataOffset',[Reflection.BindingFlags]'Instance,Public,NonPublic')
function Snapshot($param,[byte[]]$bytes) {
    $result=@{}
    foreach($row in $param.Rows) {
        if($result.ContainsKey($row.ID)) {throw "Duplicate row $($row.ID)"}
        $data=[byte[]]::new($param.DetectedSize)
        [Array]::Copy($bytes,[long]$offset.GetValue($row),$data,0,$data.Length)
        $result[$row.ID]=@{name=$row.Name;sha256=(Digest $data)}
    }
    $result
}
function Cells($row) {
    $result=@{}
    foreach($cell in $row.Cells) {$result[$cell.Def.InternalName]=$cell}
    $result
}
function ValueKey($value) {ConvertTo-Json -InputObject $value -Compress -Depth 4}
function DefinitionMap([string]$root) {
    $map=@{}
    foreach($file in Get-ChildItem -LiteralPath $root -Filter '*.xml') {
        [xml]$xml=Get-Content -LiteralPath $file.FullName -Raw
        $map[[string]$xml.PARAMDEF.ParamType]=$file.FullName
    }
    $map
}
function RestoreInlineDefaults($def,[string]$path) {
    # This AquaToolset SoulsFormats build reads field layouts but loses inline
    # "= value" XML defaults. Hydrate them explicitly before creating ER rows.
    [xml]$xml=Get-Content -LiteralPath $path -Raw
    $byName=@{}
    foreach($field in $def.Fields) {$byName[$field.InternalName]=$field}
    foreach($node in $xml.PARAMDEF.Fields.Field) {
        if([string]$node.Def -match '^\S+\s+([^\s:\[]+)(?:\s*:\s*\d+|\s*\[\s*\d+\s*\])?\s*=\s*(\S+)\s*$') {
            $field=$byName[$Matches[1]]
            if(-not $field) {continue}
            $number=[double]::Parse($Matches[2],[Globalization.CultureInfo]::InvariantCulture)
            $field.Default=[Convert]::ChangeType($number,$field.Default.GetType(),[Globalization.CultureInfo]::InvariantCulture)
        }
    }
}
$inputHash=Hash $Regulation
$nrHash=Hash $Nightreign
$aniHash=Hash $Animations
if($nrHash -ne '2570844bba290f3f3d6ffcf6d1fa493e3c6e01503f1e4f735f7f75bca104bc65') {throw 'Nightreign source differs from researched version'}
$output=[IO.Path]::GetFullPath($OutputDirectory)
$candidate=Join-Path $output 'regulation.bin'
if(Test-Path -LiteralPath $candidate) {throw 'Choose a fresh output directory'}
if($candidate -in @([IO.Path]::GetFullPath($Regulation),[IO.Path]::GetFullPath($Nightreign))) {throw 'Candidate cannot be either input'}
New-Item -ItemType Directory -Force -Path $output | Out-Null
$er=[SoulsFormats.Cryptography.RegulationDecryptor]::DecryptERRegulation($Regulation)
$nr=[SoulsFormats.Cryptography.RegulationDecryptor]::DecryptERNRRegulation($Nightreign)
$plan=[ordered]@{
    BehaviorParam=@(200100100,200100110,200100120,200100121,200100130,200100135,200100150,200100170,200100171,200100175,200100200,200100250,200100255,200100400,200100405,200100450,200100455,200100700,200100800,200100990)
    AtkParam_Npc=@(10100,10110,10120,10121,10130,10135,10150,10170,10171,10175,10200,10250,10255,10400,10405,10450,10455,10700,10800,10990)
    Bullet=@(10135,10136,10175,10176,10255,10256,10400,10405,10455,10456,10700,10800)
    SpEffectParam=@(102200,102210,707120,7011701)
}
$beforeFiles=@{}
foreach($file in $er.Files) {
    if($beforeFiles.ContainsKey($file.Name)) {throw 'Duplicate binder path'}
    $beforeFiles[$file.Name]=@{id=$file.ID;flags=[string]$file.Flags;sha256=(Digest $file.Bytes)}
}
$maps=@{ER=(DefinitionMap $erDefs);NR=(DefinitionMap $nrDefs)}
$params=@{ER=@{};NR=@{}}
$defs=@{ER=@{};NR=@{}}
$schemas=@()
$snapshots=@{}
$allNames=@($plan.Keys)+@('NpcParam')
foreach($game in @('ER','NR')) {
    $binder=if($game -eq 'ER'){$er}else{$nr}
    foreach($name in $allNames) {
        $file=Entry $binder $name
        $param=[SoulsFormats.PARAM]::Read([byte[]]$file.Bytes)
        if($game -eq 'ER') {$snapshots[$name]=Snapshot $param $file.Bytes}
        $path=$maps[$game][$param.ParamType]
        if(-not $path) {throw "Missing definition $game/$name"}
        $def=[SoulsFormats.PARAMDEF]::XmlDeserialize($path,$false,$false)
        RestoreInlineDefaults $def $path
        if($param.DetectedSize -ne $def.GetRowSize(0) -or $param.ParamType -ne $def.ParamType) {throw "Schema mismatch $game/$name"}
        $param.ApplyParamdef($def)
        $params[$game][$name]=$param
        $defs[$game][$name]=$def
        $schemas+=@{game=$game;table=$name;path=$path;sha256=(Hash $path);row_size=$param.DetectedSize;param_version=$param.ParamdefDataVersion;def_version=$def.DataVersion}
    }
}
$overrides=@()
$omissions=@{}
$expected=@{}
foreach($name in $plan.Keys) {
    $target=$params.ER[$name]
    $source=$params.NR[$name]
    $omitted=[Collections.Generic.HashSet[string]]::new()
    foreach($id in $plan[$name]) {
        if($target[$id]) {throw "Existing row collision: $name/$id; refusing to overwrite"}
        if(-not $source[$id]) {throw "Missing source row: $name/$id"}
        $row=[SoulsFormats.PARAM+Row]::new($id,('[ERDB] '+$source[$id].Name),$defs.ER[$name])
        $targetCells=Cells $row
        foreach($cell in $source[$id].Cells) {
            $field=$cell.Def.InternalName
            if($cell.Def.DisplayType -eq 'dummy8' -or $field -match '^(unknown|unk_|pad)' -or -not $targetCells.ContainsKey($field)) {
                [void]$omitted.Add($field)
                continue
            }
            $dest=$targetCells[$field]
            $value=$cell.Value
            $reason=$null
            if($name -eq 'SpEffectParam' -and $field -in @('physicsDiffence','magicDiffence','fireDiffence','thunderDiffence')) {
                if($value -ne 1) {throw "Non-neutral NR defense multiplier needs an explicit conversion: $id/$field"}
                $value=[int]0; $reason='NR neutral multiplier to ER neutral flat defense'
            } elseif($name -eq 'SpEffectParam' -and $field -eq 'spCategory') {
                if($value -lt 0 -or $value -gt 65535) {throw 'SpEffect category outside ER range'}
                $value=[uint16]$value
            } elseif($cell.Def.DisplayType -ne $dest.Def.DisplayType -or $cell.Def.BitSize -ne $dest.Def.BitSize) {
                throw "Unreviewed field layout difference: $name/$field"
            }
            if($name -eq 'AtkParam_Npc' -and $field -eq 'subCategory2' -and $value -eq 123) {
                $value=[byte]0; $reason='NR Ultimate Art is ER Magma; neutralize'
            }
            if($name -eq 'AtkParam_Npc' -and $field -eq 'atkBehaviorId' -and $value -eq 12) {
                if($id -notin @(10450,10455)) {throw 'Unexpected NR Ult Stagger row'}
                $value=[byte]0; $reason='NR Ult Stagger unavailable in ER'
            }
            if($name -eq 'SpEffectParam' -and $field -eq 'vitalSpotChangeRate') {
                if($value -ne 1 -or $dest.Value -ne -1) {throw 'Unexpected neutral weak-point correction'}
                $value=$dest.Value; $reason='Use ER default for neutral weak-point correction'
            }
            $dest.Value=$value
            if($reason) {$overrides+=@{table=$name;id=$id;field=$field;source=$cell.Value;target=$value;reason=$reason}}
        }
        $expected["$name/$id"]=@{}
        foreach($cell in $row.Cells) {$expected["$name/$id"][$cell.Def.InternalName]=ValueKey $cell.Value}
        $index=0
        while($index -lt $target.Rows.Count -and $target.Rows[$index].ID -lt $id) {$index++}
        $target.Rows.Insert($index,$row)
    }
    $omissions[$name]=@($omitted | Sort-Object)
    (Entry $er $name).Bytes=$target.Write()
}
$npc=$params.ER.NpcParam[100000]
if(-not $npc) {throw 'Restore NpcParam100000 before porting combat data'}
$npcBefore=@{}
foreach($cell in $npc.Cells) {$npcBefore[$cell.Def.InternalName]=ValueKey $cell.Value}
$oldVariation=$npc['behaviorVariationId'].Value
if($oldVariation -ne 40700) {throw "Npc behavior variation differs from verified load/control baseline: $oldVariation"}
$npc['behaviorVariationId'].Value=[int]100
$npc.Name='[ERDB] c0010 - ER compatible combat'
(Entry $er 'NpcParam').Bytes=$params.ER.NpcParam.Write()
[SoulsFormats.Cryptography.RegulationDecryptor]::EncryptERRegulation($candidate,$er)
$verified=[SoulsFormats.Cryptography.RegulationDecryptor]::DecryptERRegulation($candidate)
if($verified.Files.Count -ne $beforeFiles.Count -or $verified.Version -ne $er.Version) {throw 'Binder metadata changed'}
$changed=@()
foreach($file in $verified.Files) {
    $before=$beforeFiles[$file.Name]
    if(-not $before -or $before.id -ne $file.ID -or $before.flags -ne [string]$file.Flags) {throw 'Binder entry metadata changed'}
    if($before.sha256 -ne (Digest $file.Bytes)) {$changed+=[IO.Path]::GetFileNameWithoutExtension($file.Name)}
}
if((@($changed | Sort-Object) -join ',') -ne (@($allNames | Sort-Object) -join ',')) {throw 'Changed tables differ from allowlist'}
$preserved=0
$readback=@{}
foreach($name in $allNames) {
    $file=Entry $verified $name
    $param=[SoulsFormats.PARAM]::Read([byte[]]$file.Bytes)
    $actual=Snapshot $param $file.Bytes
    $before=$snapshots[$name]
    $count=if($name -eq 'NpcParam'){0}else{$plan[$name].Count}
    if($actual.Count -ne $before.Count+$count) {throw "Unexpected row count $name"}
    foreach($id in $before.Keys) {
        if($name -eq 'NpcParam' -and $id -eq 100000) {continue}
        if(-not $actual.ContainsKey($id) -or $actual[$id].sha256 -ne $before[$id].sha256 -or $actual[$id].name -cne $before[$id].name) {throw "Unplanned existing-row change: $name/$id"}
        $preserved++
    }
    $param.ApplyParamdef($defs.ER[$name])
    $readback[$name]=$param
    if($name -eq 'NpcParam') {
        foreach($cell in $param[100000].Cells) {
            $field=$cell.Def.InternalName
            if($field -eq 'behaviorVariationId') {if($cell.Value -ne 100){throw 'Npc routing did not read back'}}
            elseif((ValueKey $cell.Value) -cne $npcBefore[$field]) {throw "Unexpected Npc field change: $field"}
        }
        if($param[100000].Name -cne $npc.Name) {throw 'Npc name readback mismatch'}
    } else {
        foreach($id in $plan[$name]) {
            if($param[$id].Name -cne $params.ER[$name][$id].Name) {throw 'Imported name changed'}
            foreach($cell in $param[$id].Cells) {
                if((ValueKey $cell.Value) -cne $expected["$name/$id"][$cell.Def.InternalName]) {throw "Imported value changed: $name/$id/$($cell.Def.InternalName)"}
            }
        }
    }
}
$referenceChecks=[Collections.Generic.List[object]]::new()
function CheckRef([string]$from,[string]$table,[int]$id,[int]$minimum=0) {
    if($id -lt $minimum) {return}
    if(-not $readback.ContainsKey($table)) {$readback[$table]=[SoulsFormats.PARAM]::Read([byte[]](Entry $verified $table).Bytes)}
    if(-not $readback[$table][$id]) {throw "Missing reference: $from -> $table/$id"}
    $referenceChecks.Add(@{from=$from;table=$table;id=$id})
}
foreach($id in $plan.BehaviorParam) {
    $row=$readback.BehaviorParam[$id]
    if($row['variationId'].Value -ne 100) {throw 'Wrong behavior group'}
    $refTable=switch($row['refType'].Value){0{'AtkParam_Npc'} 1{'Bullet'} 2{'SpEffectParam'} default{throw 'Unsupported behavior reference'}}
    CheckRef "Behavior/$id" $refTable $row['refId'].Value
}
foreach($id in $plan.AtkParam_Npc) {
    $row=$readback.AtkParam_Npc[$id]
    foreach($f in @('spEffectId0','spEffectId1','spEffectId2','spEffectId3','spEffectId4')) {CheckRef "Atk/$id/$f" 'SpEffectParam' $row[$f].Value}
    foreach($f in @('AppearAiSoundId','HitAiSoundId')) {CheckRef "Atk/$id/$f" 'AiSoundParam' $row[$f].Value 1}
    foreach($f in @('decalId1','decalId2')) {CheckRef "Atk/$id/$f" 'DecalParam' $row[$f].Value}
    CheckRef "Atk/$id/overwriteAttackElementCorrectId" 'AttackElementCorrectParam' $row['overwriteAttackElementCorrectId'].Value
    CheckRef "Atk/$id/finalDamageRateId" 'FinalDamageRateParam' $row['finalDamageRateId'].Value
}
foreach($id in $plan.Bullet) {
    $row=$readback.Bullet[$id]
    CheckRef "Bullet/$id/attack" 'AtkParam_Npc' $row['atkId_Bullet'].Value
    foreach($f in @('HitBulletID','intervalCreateBulletId')) {CheckRef "Bullet/$id/$f" 'Bullet' $row[$f].Value}
    foreach($f in @('spEffectIDForShooter','spEffectId0','spEffectId1','spEffectId2','spEffectId3','spEffectId4')) {CheckRef "Bullet/$id/$f" 'SpEffectParam' $row[$f].Value 1}
    CheckRef "Bullet/$id/AI" 'NpcThinkParam' $row['autoSearchNPCThinkID'].Value 1
    CheckRef "Bullet/$id/limit" 'BulletCreateLimitParam' $row['createLimitGroupId'].Value 1
}
foreach($id in $plan.SpEffectParam) {
    $row=$readback.SpEffectParam[$id]
    if($row['stateInfo'].Value -ne 0) {throw 'Imported effect is not ER generic'}
    foreach($f in @('replaceSpEffectId','cycleOccurrenceSpEffectId','atkOccurrenceSpEffectId','accumuOverFireId','accumuUnderFireId','spiritDeathSpEffectId')) {CheckRef "SpEffect/$id/$f" 'SpEffectParam' $row[$f].Value 1}
    foreach($f in @('vfxId','vfxId1','vfxId2','vfxId3','vfxId4','vfxId5','vfxId6','vfxId7')) {CheckRef "SpEffect/$id/$f" 'SpEffectVfxParam' $row[$f].Value}
    CheckRef "SpEffect/$id/behavior" 'BehaviorParam' $row['behaviorId'].Value
    CheckRef "SpEffect/$id/AI-sound" 'AiSoundParam' $row['AppearAiSoundId'].Value 1
    CheckRef "SpEffect/$id/foot" 'FootSfxParam' $row['addFootEffectSfxId'].Value
}
$ani=[SoulsFormats.BND4]::Read($Animations)
$taeFiles=@($ani.Files | Where-Object Name -like '*.tae')
if($taeFiles.Count -ne 1) {throw 'Expected exactly one c0010 TAE'}
$tae=[SoulsFormats.TAE]::Read([byte[]]$taeFiles[0].Bytes)
$taeReferences=@()
foreach($animation in $tae.Animations) {foreach($event in $animation.Events) {
    if($event.Type -notin @(1,2,66,67,401)) {continue}
    $bytes=$event.GetParameterBytes($false)
    if($event.Type -in @(1,2)) {
        $judge=[BitConverter]::ToInt32($bytes,8)
        $id=200100000+$judge
        CheckRef "TAE/$($animation.ID)/$($event.Type)" 'BehaviorParam' $id
        $behavior=$readback.BehaviorParam[$id]
        if($behavior['behaviorJudgeId'].Value -ne $judge -or $behavior['refType'].Value -ne ($event.Type-1)) {throw 'TAE behavior reference has wrong judge/type'}
        $taeReferences+=@{animation=$animation.ID;type=$event.Type;behavior=$id}
    } else {
        $id=[BitConverter]::ToInt32($bytes,0)
        CheckRef "TAE/$($animation.ID)/$($event.Type)" 'SpEffectParam' $id
    }
}}
if((Hash $Regulation) -ne $inputHash -or (Hash $Nightreign) -ne $nrHash -or (Hash $Animations) -ne $aniHash) {throw 'Input changed during candidate preparation'}
$report=[ordered]@{
    kind='c0010-combat-param-port';spec='spec-combat-params.md@1.0';verification='PASS';runtime_tested=$false
    source=$Regulation;source_sha256=$inputHash;nightreign=$Nightreign;nightreign_sha256=$nrHash
    animations=$Animations;animations_sha256=$aniHash;candidate=$candidate;candidate_sha256=(Hash $candidate)
    added=$plan;added_count=56;npc_change=@{id=100000;field='behaviorVariationId';before=$oldVariation;after=100;name=$npc.Name}
    changed_tables=$changed;unchanged_tables=$beforeFiles.Count-$changed.Count;preserved_existing_rows=$preserved
    schemas=$schemas;overrides=$overrides;omitted_source_fields=$omissions
    excluded_rows=@{BehaviorParam=@(200100201);SpEffectParam=@(707110,707130,705012)}
    reference_checks=$referenceChecks;tae_attack_notifications=$taeReferences
    limits=@('No NR immortality or 70% reduction','No NR relic/hero/grapple trigger implementation','No NR ultimate-specific stagger','No runtime hit/damage verification','SFX asset compatibility not established')
}
$report | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $output 'port-report.json') -Encoding utf8
@{candidate=$candidate;sha256=$report.candidate_sha256;added=56;preserved_existing_rows=$preserved;unchanged_tables=$report.unchanged_tables;reference_checks=$referenceChecks.Count;tae_attack_notifications=$taeReferences.Count;verification='PASS'} | ConvertTo-Json
