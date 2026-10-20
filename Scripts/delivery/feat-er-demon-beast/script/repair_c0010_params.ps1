param(
    [string]$Regulation='F:\GoldenAge\_GoldenAge\GA\regulation.bin',
    [Parameter(Mandatory=$true)][string]$Reference,
    [Parameter(Mandatory=$true)][string]$OutputDirectory
)
# Prepare a missing-row-only merge; installed inputs are never written here.
$ErrorActionPreference='Stop'
$deps='D:\_Ugit\FSTool\FSModify\UnpackingTools\AquaToolset\Aqua.Toolset_2026.7.1\net9.0-windows7.0'
$defs='D:\_Ugit\FSTool\FSModify\UnpackingTools\WitchyBND\WitchyBND-3.0.1.0-win-x64\Assets\Paramdex\ER\Defs'
Add-Type -Path (Join-Path $deps 'BouncyCastle.Cryptography.dll')
Add-Type -Path (Join-Path $deps 'SoulsFormats.dll')
[void][Runtime.InteropServices.NativeLibrary]::Load('F:\SteamLibrary\steamapps\common\ELDEN RING\Game\oo2core_6_win64.dll')
function Hash([string]$path) {(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()}
function Digest([byte[]]$bytes) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()}
function Entry($bnd,[string]$name) {
    $found=@($bnd.Files | Where-Object {[IO.Path]::GetFileNameWithoutExtension($_.Name) -eq $name})
    if($found.Count -ne 1) {throw "Expected one table: $name"}
    $found[0]
}
$offset=[SoulsFormats.PARAM+Row].GetField('DataOffset',[Reflection.BindingFlags]'Instance,Public,NonPublic')
function Snapshot($param,[byte[]]$bytes) {
    $rows=@{}
    foreach($row in $param.Rows) {
        if($rows.ContainsKey($row.ID)) {throw "Duplicate row: $($row.ID)"}
        $data=[byte[]]::new($param.DetectedSize)
        [Array]::Copy($bytes,[long]$offset.GetValue($row),$data,0,$data.Length)
        $rows[$row.ID]=@{name=$row.Name;sha256=(Digest $data)}
    }
    $rows
}
$referenceHash=Hash $Reference
if($referenceHash -ne '07444d99aef7166a2ad9e8a505842134518ca62851bdeaaee20e6f6149abc4cd') {throw 'Reference is not the previously recorded c0010 parameter package'}
$inputHash=Hash $Regulation
$output=[IO.Path]::GetFullPath($OutputDirectory)
$candidate=Join-Path $output 'regulation.bin'
if($candidate -eq [IO.Path]::GetFullPath($Regulation) -or $candidate -eq [IO.Path]::GetFullPath($Reference)) {throw 'Candidate must not overwrite either input'}
if(Test-Path -LiteralPath $candidate) {throw 'Choose a fresh candidate directory'}
New-Item -ItemType Directory -Force -Path $output | Out-Null
$bnd=[SoulsFormats.Cryptography.RegulationDecryptor]::DecryptERRegulation($Regulation)
$donor=[SoulsFormats.Cryptography.RegulationDecryptor]::DecryptERRegulation($Reference)
$entries=@{}
foreach($entry in $bnd.Files) {
    if($entries.ContainsKey($entry.Name)) {throw 'Duplicate binder path'}
    $entries[$entry.Name]=@{id=$entry.ID;flags=[string]$entry.Flags;sha256=(Digest $entry.Bytes)}
}
$snapshots=@{}
$added=@()
$unchanged=@()
foreach($name in @('NpcParam','NpcThinkParam')) {
    $file=Entry $bnd $name
    $param=[SoulsFormats.PARAM]::Read([byte[]]$file.Bytes)
    $snapshots[$name]=Snapshot $param $file.Bytes
    $refFile=Entry $donor $name
    $refParam=[SoulsFormats.PARAM]::Read([byte[]]$refFile.Bytes)
    $refRows=Snapshot $refParam $refFile.Bytes
    if(-not $refRows.ContainsKey(100000)) {throw "Reference row missing: $name"}
    if($snapshots[$name].ContainsKey(100000)) {
        $unchanged+=@{table=$name;id=100000;status='present; preserved without replacement'}
        continue
    }
    if($param.ParamType -ne $refParam.ParamType -or $param.DetectedSize -ne $refParam.DetectedSize -or $param.ParamdefDataVersion -ne $refParam.ParamdefDataVersion) {throw "Reference schema mismatch: $name"}
    $def=[SoulsFormats.PARAMDEF]::XmlDeserialize((Join-Path $defs "$name.xml"),$false,$false)
    if(-not $param.ApplyParamdefCarefully($def) -or -not $refParam.ApplyParamdefCarefully($def)) {throw "Paramdef mismatch: $name"}
    $source=$refParam[100000]
    $row=[SoulsFormats.PARAM+Row]::new(100000,$source.Name,$def)
    foreach($cell in $source.Cells) {$row[$cell.Def.InternalName].Value=$cell.Value}
    $index=0
    while($index -lt $param.Rows.Count -and $param.Rows[$index].ID -lt 100000) {$index++}
    $param.Rows.Insert($index,$row)
    $file.Bytes=$param.Write()
    $added+=@{table=$name;id=100000;name=$source.Name;expected_sha256=$refRows[100000].sha256}
}
if($added.Count -eq 0) {throw 'Both rows already exist; no repair required'}
[SoulsFormats.Cryptography.RegulationDecryptor]::EncryptERRegulation($candidate,$bnd)
$verify=[SoulsFormats.Cryptography.RegulationDecryptor]::DecryptERRegulation($candidate)
if($verify.Files.Count -ne $entries.Count) {throw 'Binder count changed'}
$changed=@()
foreach($file in $verify.Files) {
    $before=$entries[$file.Name]
    if($null -eq $before -or $before.id -ne $file.ID -or $before.flags -ne [string]$file.Flags) {throw 'Binder metadata changed'}
    if($before.sha256 -ne (Digest $file.Bytes)) {$changed+=[IO.Path]::GetFileNameWithoutExtension($file.Name)}
}
if((@($changed | Sort-Object) -join ',') -ne (@($added.table | Sort-Object) -join ',')) {throw 'Changed tables exceed the missing-row merge'}
$preserved=0
foreach($name in @('NpcParam','NpcThinkParam')) {
    $file=Entry $verify $name
    $param=[SoulsFormats.PARAM]::Read([byte[]]$file.Bytes)
    $actual=Snapshot $param $file.Bytes
    $before=$snapshots[$name]
    $planned=@($added | Where-Object table -eq $name)
    if($actual.Count -ne ($before.Count+$planned.Count)) {throw "Row count changed unexpectedly: $name"}
    foreach($id in $before.Keys) {
        if(-not $actual.ContainsKey($id) -or $actual[$id].sha256 -ne $before[$id].sha256 -or $actual[$id].name -cne $before[$id].name) {throw "Existing row changed: $name[$id]"}
        $preserved++
    }
    foreach($expected in $planned) {
        if($actual[100000].sha256 -ne $expected.expected_sha256 -or $actual[100000].name -cne $expected.name) {throw "Imported row differs from reference: $name"}
    }
}
if((Hash $Regulation) -ne $inputHash -or (Hash $Reference) -ne $referenceHash) {throw 'Input changed during preparation'}
$report=[ordered]@{kind='c0010-missing-row-repair';source=$Regulation;source_sha256=$inputHash;reference=$Reference;reference_sha256=$referenceHash;candidate=$candidate;candidate_sha256=(Hash $candidate);added=$added;already_present=$unchanged;changed_tables=$changed;unchanged_tables=$entries.Count-$changed.Count;preserved_existing_rows=$preserved;verification='PASS';runtime_tested=$false}
$report | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $output 'repair-report.json') -Encoding utf8
$report | ConvertTo-Json -Depth 10
