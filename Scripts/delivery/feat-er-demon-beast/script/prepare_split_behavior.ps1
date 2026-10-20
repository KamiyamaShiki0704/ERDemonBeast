param(
    [string]$GoldenAgeAssets = 'F:/GoldenAge/_GoldenAge/GA',
    [string]$DllRoot = 'F:/GoldenAge/_GoldenAge/dll/demon_beast',
    [Parameter(Mandatory=$true)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
$project = (Resolve-Path "$PSScriptRoot/../../../..").Path
$out = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $out) { throw 'Choose a fresh output directory.' }
$deps = 'D:/_Ugit/FSTool/FSModify/UnpackingTools/AquaToolset/Aqua.Toolset_2026.7.1/net9.0-windows7.0'
$cli = 'D:/_Ugit/FSTool/FSModify/AnimsTools/_Tools/HKLibCLI/HKLibCLI.v0.1.3-NR/net7.0/HKLib.CLI.exe'
Add-Type -Path (Join-Path $deps 'SoulsFormats.dll')
$oodle = [Runtime.InteropServices.NativeLibrary]::Load('F:/SteamLibrary/steamapps/common/ELDEN RING/Game/oo2core_6_win64.dll')
function HashBytes([byte[]]$bytes) { [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant() }
function HashFile([string]$path) { (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() }
$relative = @('chr/c0010.behbnd.dcx', 'action/script/c9997.hks', 'action/script/d0010.hks')
$before = @{}
foreach ($r in $relative) { $before[$r] = HashFile (Join-Path $GoldenAgeAssets $r) }
$bnd = [SoulsFormats.BND4]::Read((Join-Path $GoldenAgeAssets $relative[0]))
if ($bnd.Files.Count -ne 4 -or $bnd.Files[1].Name -notlike '*Characters\c0010.hkx') { throw 'Unexpected source binder topology' }
New-Item -ItemType Directory -Path $out,(Join-Path $out 'chr') | Out-Null
$entries = @($bnd.Files | ForEach-Object { @{name=$_.Name;id=$_.ID;flags=[string]$_.Flags;hash=(HashBytes $_.Bytes)} })
[IO.File]::WriteAllBytes((Join-Path $out 'character-before.hkx'), $bnd.Files[1].Bytes)
& $cli (Join-Path $out 'character-before.hkx')
if ($LASTEXITCODE -ne 0) { throw 'Character extraction failed' }
& python "$PSScriptRoot/split_behavior.py" prepare "$out/character-before.xml" "$GoldenAgeAssets/action/script/c9997.hks" "$project/assets/action/script/c0010-er-adapter.hks" $out
if ($LASTEXITCODE -ne 0) { throw 'Split preparation failed' }
& $cli "$out/character-split.xml"
if ($LASTEXITCODE -ne 0) { throw 'Character serialization failed' }
Copy-Item -LiteralPath "$out/character-split.hkx" -Destination "$out/character-roundtrip.hkx"
& $cli "$out/character-roundtrip.hkx"
if ($LASTEXITCODE -ne 0) { throw 'Character roundtrip failed' }
& python "$PSScriptRoot/split_behavior.py" verify "$out/character-before.xml" "$out/character-split.xml" "$out/character-roundtrip.xml"
if ($LASTEXITCODE -ne 0) { throw 'Full character equivalence failed' }
$bnd.Files[1].Bytes = [IO.File]::ReadAllBytes("$out/character-split.hkx")
$candidate = Join-Path $out $relative[0]
$bnd.Write($candidate)
$verify = [SoulsFormats.BND4]::Read($candidate)
if ($verify.Files.Count -ne $entries.Count) { throw 'Binder entry count changed' }
for ($i=0; $i -lt $entries.Count; $i++) {
    $f=$verify.Files[$i]; $ref=$entries[$i]
    if ($f.Name -cne $ref.name -or $f.ID -ne $ref.id -or [string]$f.Flags -cne $ref.flags) { throw 'Binder entry metadata changed' }
    $expected = if ($i -eq 1) { HashFile "$out/character-split.hkx" } else { $ref.hash }
    if ((HashBytes $f.Bytes) -ne $expected) { throw "Binder entry data mismatch: $i" }
}
$files = @($relative | ForEach-Object { @{relative=$_; before=$before[$_]; after=(HashFile (Join-Path $out $_))} })
$protected = @('regulation.bin','chr/c0010.anibnd.dcx','action/script/c0010.hks','action/eventnameid.txt','action/statenameid.txt','action/variablenameid.txt')
$guards = @($protected | ForEach-Object { @{relative=$_;hash=(HashFile (Join-Path $GoldenAgeAssets $_))} })
$dllGuards = @(@('er_demon_beast.dll','er-demon-beast.toml') | ForEach-Object { @{relative=$_;hash=(HashFile (Join-Path $DllRoot $_))} })
@{format=1;revision='split-hks-1';result='PASS_ASSET_STRUCTURE';files=$files;protected=$guards;dll_protected=$dllGuards;binder_entries=$entries;luaFiles=@('c9997.hks','d0010.hks','modifier.hks','c0010.hks','c0010.hks');roundtrip='PASS: 3 entries byte-identical; complete character graph equivalent except luaFiles';adapter_body='byte-identical after newline normalization; header comments only';runtime_tested=$false} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath "$out/split-build.json" -Encoding utf8
Write-Output "$out/split-build.json"
