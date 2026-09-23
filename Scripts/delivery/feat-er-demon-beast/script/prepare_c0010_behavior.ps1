param(
    [string]$GoldenAgeAssets = 'F:/GoldenAge/_GoldenAge/GA',
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [Parameter(Mandatory=$true)][string]$Adapter
)
# Offline only: source assets are read, all outputs go to a fresh project directory.
$ErrorActionPreference = 'Stop'
$out = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $out) { throw 'Choose a fresh output directory.' }
$deps = 'D:/_Ugit/FSTool/FSModify/UnpackingTools/AquaToolset/Aqua.Toolset_2026.7.1/net9.0-windows7.0'
$cli = 'D:/_Ugit/FSTool/FSModify/AnimsTools/_Tools/HKLibCLI/HKLibCLI.v0.1.3-NR/net7.0/HKLib.CLI.exe'
Add-Type -Path (Join-Path $deps 'SoulsFormats.dll')
$oodle = [Runtime.InteropServices.NativeLibrary]::Load('F:/SteamLibrary/steamapps/common/ELDEN RING/Game/oo2core_6_win64.dll')
function HashBytes([byte[]]$b) { [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($b)).ToLowerInvariant() }
$source = Join-Path $GoldenAgeAssets 'chr/c0010.behbnd.dcx'
$beforeHash = (Get-FileHash -LiteralPath $source).Hash.ToLowerInvariant()
$bnd = [SoulsFormats.BND4]::Read($source)
if ($bnd.Files.Count -ne 4 -or $bnd.Files[1].Name -notlike '*Characters\c0010.hkx') { throw 'Unexpected source binder topology' }
$extract = Join-Path $out 'extracted'
New-Item -ItemType Directory -Force -Path $extract,(Join-Path $out 'chr') | Out-Null
$original = @()
for($i=0; $i -lt $bnd.Files.Count; $i++) {
    $file = $bnd.Files[$i]
    $original += @{name=$file.Name;id=$file.ID;flags=[string]$file.Flags;hash=(HashBytes $file.Bytes)}
    $path = Join-Path $extract "$i.hkx"
    [IO.File]::WriteAllBytes($path,$file.Bytes)
    & $cli $path
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath (Join-Path $extract "$i.xml"))) { throw "HKX parse failed: $i" }
}
& python (Join-Path $PSScriptRoot 'behavior_assets.py') $extract $GoldenAgeAssets $Adapter $out
if ($LASTEXITCODE -ne 0) { throw 'Behavior/name table preparation failed' }
$bnd.Files[1].Bytes = [IO.File]::ReadAllBytes((Join-Path $out 'character-patched.hkx'))
$candidate = Join-Path $out 'chr/c0010.behbnd.dcx'
$bnd.Write($candidate)
$verify = [SoulsFormats.BND4]::Read($candidate)
if ($verify.Files.Count -ne $original.Count) { throw 'Binder entry count changed' }
for($i=0; $i -lt $verify.Files.Count; $i++) {
    $file=$verify.Files[$i]; $ref=$original[$i]
    if($file.Name -cne $ref.name -or $file.ID -ne $ref.id -or [string]$file.Flags -cne $ref.flags) { throw 'Binder metadata changed' }
    $expected = if($i -eq 1) { HashBytes ([IO.File]::ReadAllBytes((Join-Path $out 'character-patched.hkx'))) } else { $ref.hash }
    if((HashBytes $file.Bytes) -ne $expected) { throw "Roundtrip data mismatch: $i" }
}
$anibnd = [SoulsFormats.BND4]::Read((Join-Path $GoldenAgeAssets 'chr/c0010.anibnd.dcx'))
$taeFile = @($anibnd.Files | Where-Object Name -like '*.tae')
if($taeFile.Count -ne 1) { throw 'Expected one c0010 TAE' }
$ids = @([SoulsFormats.TAE]::Read([byte[]]$taeFile[0].Bytes).Animations | ForEach-Object ID)
foreach($id in @(3000,3001,3002,3003,3005,3007,3010,3030,3035,6000)) { if($id -notin $ids) { throw "Missing TAE: $id" } }
# Parse the actual fixed-length HKX patch and verify its script array structurally.
& $cli (Join-Path $out 'character-patched.hkx')
if($LASTEXITCODE -ne 0) { throw 'Patched HKX parse failed' }
[xml]$xml = Get-Content -LiteralPath (Join-Path $out 'character-patched.xml') -Raw
$names = @($xml.SelectNodes("//field[@name='luaFiles']/array/string") | ForEach-Object value)
if(($names -join ',') -cne 'd0010.hks,modifier.hks,c0010.hks,c0010.hks') { throw 'Patched luaFiles contract failed' }
$reportPath = Join-Path $out 'behavior-build.json'
$report = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
$report.files += @{relative='chr/c0010.behbnd.dcx';before=$beforeHash;after=(Get-FileHash -LiteralPath $candidate).Hash.ToLowerInvariant()}
$report | Add-Member binder_entries $original
$report | Add-Member roundtrip 'PASS; 3 entries byte-identical; character entry fixed-length script rename only'
$report | Add-Member verified_tae_ids @(3000,3001,3002,3003,3005,3007,3010,3030,3035,6000)
$report | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $reportPath -Encoding utf8
Write-Output $reportPath
