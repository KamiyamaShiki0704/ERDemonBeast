param(
    [string]$Regulation = 'F:\GoldenAge\_GoldenAge\GA\regulation.bin',
    [string]$NrRegulation = 'F:\SteamLibrary\steamapps\common\ELDEN RING NIGHTREIGN\Game\regulation.bin',
    [Parameter(Mandatory=$true)][string]$OutputDirectory
)
# Offline preparation only. Never writes source/installed regulation or starts a game.
$ErrorActionPreference = 'Stop'
$deps = 'D:\_Ugit\FSTool\FSModify\UnpackingTools\AquaToolset\Aqua.Toolset_2026.7.1\net9.0-windows7.0'
$defs = 'D:\_Ugit\FSTool\FSModify\UnpackingTools\WitchyBND\WitchyBND-3.0.1.0-win-x64\Assets\Paramdex'
Add-Type -Path (Join-Path $deps 'BouncyCastle.Cryptography.dll')
Add-Type -Path (Join-Path $deps 'SoulsFormats.dll')
$oodleHandle = [Runtime.InteropServices.NativeLibrary]::Load('F:\SteamLibrary\steamapps\common\ELDEN RING\Game\oo2core_6_win64.dll')
$output = [IO.Path]::GetFullPath($OutputDirectory)
$candidate = Join-Path $output 'regulation.bin'
if ([IO.Path]::GetFullPath($Regulation) -eq $candidate) { throw 'Output must not overwrite input' }
if (Test-Path -LiteralPath $candidate) { throw 'Candidate already exists; choose a new output directory' }
New-Item -ItemType Directory -Force -Path $output | Out-Null
function Digest([byte[]]$bytes) { [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)) }
function Entry($bnd, [string]$name) {
    $matches = @($bnd.Files | Where-Object { [IO.Path]::GetFileNameWithoutExtension($_.Name) -eq $name })
    if ($matches.Count -ne 1) { throw "Expected one table: $name" }
    return $matches[0]
}
$offsetField = [SoulsFormats.PARAM+Row].GetField('DataOffset', [Reflection.BindingFlags]'Instance,Public,NonPublic')
function RowSnapshot($param, [byte[]]$data) {
    $result = @{}
    foreach ($row in $param.Rows) {
        if ($result.ContainsKey($row.ID)) { throw "Duplicate row: $($row.ID)" }
        $bytes = [byte[]]::new($param.DetectedSize)
        [Array]::Copy($data, [long]$offsetField.GetValue($row), $bytes, 0, $bytes.Length)
        $result[$row.ID] = @{name=$row.Name;sha256=(Digest $bytes)}
    }
    return $result
}
$inputHash = (Get-FileHash -LiteralPath $Regulation).Hash
$bnd = [SoulsFormats.Cryptography.RegulationDecryptor]::DecryptERRegulation($Regulation)
$nr = [SoulsFormats.Cryptography.RegulationDecryptor]::DecryptERNRRegulation($NrRegulation)
$originalEntries = @{}
foreach ($file in $bnd.Files) { $originalEntries[$file.Name] = @{id=$file.ID;flags=[string]$file.Flags;sha256=(Digest $file.Bytes)} }
$snapshots = @{}
$added = @()
foreach ($name in @('NpcParam','NpcThinkParam')) {
    $file = Entry $bnd $name
    $param = [SoulsFormats.PARAM]::Read([byte[]]$file.Bytes)
    $snapshots[$name] = RowSnapshot $param $file.Bytes
    $def = [SoulsFormats.PARAMDEF]::XmlDeserialize((Join-Path $defs "ER\Defs\$name.xml"), $false, $false)
    if (-not $param.ApplyParamdefCarefully($def)) { throw "ER schema mismatch: $name" }
    if ($null -ne $param[100000]) { throw "Refusing to replace existing $name[100000]" }
    $donor = $param[40700000]
    if ($null -eq $donor) { throw "Missing validated ER donor: $name[40700000]" }
    $row = [SoulsFormats.PARAM+Row]::new(100000, '[ERDB] c0010 load/control test - combat not ported', $def)
    foreach ($cell in $donor.Cells) { $row[$cell.Def.InternalName].Value = $cell.Value }
    $overrides = [ordered]@{}
    if ($name -eq 'NpcParam') {
        $nrFile = Entry $nr $name
        $nrParam = [SoulsFormats.PARAM]::Read([byte[]]$nrFile.Bytes)
        $nrDef = [SoulsFormats.PARAMDEF]::XmlDeserialize((Join-Path $defs "NR\Defs\$name.xml"), $false, $false)
        # NR data v10 vs installed v9 schema: read only the whitelisted physical
        # fields after matching row size/type and independent KB anchors.
        # Never serialize NR cells into ER wholesale or import NR engine flags.
        if ($nrParam.ParamType -ne $nrDef.ParamType -or $nrParam.DetectedSize -ne $nrDef.GetRowSize(0)) { throw 'NR physical-field layout mismatch' }
        $nrParam.ApplyParamdef($nrDef)
        $source = $nrParam[100000]
        if ($null -eq $source -or $source['hp'].Value -ne 63936 -or [Math]::Abs($source['hitHeight'].Value - 2.6) -gt 0.001 -or [Math]::Abs($source['hitRadius'].Value - 1.3) -gt 0.001 -or $source['weight'].Value -ne 300) { throw 'NR physical-field anchors changed' }
        foreach ($field in @('hp','hitHeight','hitRadius','hitYOffset','chrHitHeight','chrHitRadius','weight','turnVellocity','lockDist')) {
            $row[$field].Value = $source[$field].Value
            $overrides[$field] = $row[$field].Value
        }
        # Wolf-specific effects/drops are not part of this controllable form.
        foreach ($cell in $row.Cells) {
            if ($cell.Def.InternalName -match '^spEffectID\d+$|^itemLotId') {
                $cell.Value = -1
                $overrides[$cell.Def.InternalName] = $cell.Value
            }
        }
    }
    $index = 0
    while ($index -lt $param.Rows.Count -and $param.Rows[$index].ID -lt 100000) { $index++ }
    $param.Rows.Insert($index, $row)
    $file.Bytes = $param.Write()
    $added += @{table=$name;id=100000;donor=40700000;overrides=$overrides;scope='dedicated ER load/control test row; attack and AI references retained from ER donor'}
}
[SoulsFormats.Cryptography.RegulationDecryptor]::EncryptERRegulation($candidate, $bnd)
$verify = [SoulsFormats.Cryptography.RegulationDecryptor]::DecryptERRegulation($candidate)
if ($verify.Files.Count -ne $originalEntries.Count) { throw 'Table count changed' }
$changed = @()
foreach ($file in $verify.Files) {
    $before = $originalEntries[$file.Name]
    if ($null -eq $before -or $before.id -ne $file.ID -or $before.flags -ne [string]$file.Flags) { throw 'Binder entry metadata changed' }
    if ($before.sha256 -ne (Digest $file.Bytes)) { $changed += [IO.Path]::GetFileNameWithoutExtension($file.Name) }
}
if (@($changed | Sort-Object) -join ',' -ne 'NpcParam,NpcThinkParam') { throw "Unexpected changed tables: $changed" }
$preserved = 0
foreach ($name in @('NpcParam','NpcThinkParam')) {
    $file = Entry $verify $name
    $param = [SoulsFormats.PARAM]::Read([byte[]]$file.Bytes)
    $actual = RowSnapshot $param $file.Bytes
    $before = $snapshots[$name]
    if ($actual.Count -ne $before.Count + 1 -or -not $actual.ContainsKey(100000)) { throw "Wrong inserted row: $name" }
    foreach ($id in $before.Keys) {
        if ($actual[$id].sha256 -ne $before[$id].sha256 -or $actual[$id].name -cne $before[$id].name) { throw "Existing row changed: $name[$id]" }
        $preserved++
    }
    $def = [SoulsFormats.PARAMDEF]::XmlDeserialize((Join-Path $defs "ER\Defs\$name.xml"), $false, $false)
    if (-not $param.ApplyParamdefCarefully($def)) { throw "Readback schema failed: $name" }
    $planned = $added | Where-Object table -eq $name
    foreach ($field in $planned.overrides.Keys) { if ($param[100000][$field].Value -ne $planned.overrides[$field]) { throw "Readback override mismatch: $field" } }
}
if ((Get-FileHash -LiteralPath $Regulation).Hash -ne $inputHash) { throw 'Input changed during preparation' }
$report = [ordered]@{kind='c0010-param-candidate';source=$Regulation;source_sha256=$inputHash;nr_source=$NrRegulation;nr_source_sha256=(Get-FileHash -LiteralPath $NrRegulation).Hash;nr_schema_note='v10 row / v9 schema, equal size; only nine physical fields imported after KB anchor checks';candidate=$candidate;candidate_sha256=(Get-FileHash -LiteralPath $candidate).Hash;changed_tables=$changed;preserved_existing_rows=$preserved;added=$added;verification='PASS';runtime_tested=$false}
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $output 'param-patch-report.json') -Encoding utf8
[pscustomobject]$report | Select-Object candidate,candidate_sha256,changed_tables,preserved_existing_rows,verification | ConvertTo-Json
