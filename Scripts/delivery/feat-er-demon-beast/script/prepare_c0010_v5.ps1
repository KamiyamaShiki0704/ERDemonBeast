param(
    [string]$GoldenAgeAssets = 'F:/GoldenAge/_GoldenAge/GA',
    [Parameter(Mandatory=$true)][string]$OutputDirectory
)
# Read-only against the game. Rebuild from the installed v4 files and verify roundtrip.
$ErrorActionPreference='Stop'
$out=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $out) {throw 'Choose a fresh output directory'}
New-Item -ItemType Directory -Path $out,(Join-Path $out 'chr') | Out-Null
Add-Type -Path 'D:/_Ugit/FSTool/FSModify/UnpackingTools/AquaToolset/Aqua.Toolset_2026.7.1/net9.0-windows7.0/SoulsFormats.dll'
$oodle=[Runtime.InteropServices.NativeLibrary]::Load('F:/SteamLibrary/steamapps/common/ELDEN RING/Game/oo2core_6_win64.dll')
$cli='D:/_Ugit/FSTool/FSModify/AnimsTools/_Tools/HKLibCLI/HKLibCLI.v0.1.3-NR/net7.0/HKLib.CLI.exe'
function ConvertHkx([string]$path) { & $cli $path; if($LASTEXITCODE -ne 0) {throw 'HKLib conversion failed'} }
function HashBytes([byte[]]$bytes) { [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)) }
$bnd=[SoulsFormats.BND4]::Read((Join-Path $GoldenAgeAssets 'chr/c0010.behbnd.dcx'))
if($bnd.Files.Count -ne 4 -or $bnd.Files[3].Name -notlike '*Behaviors\c9997.hkx') {throw 'Unexpected graph binder'}
$before=@($bnd.Files | ForEach-Object { @{name=$_.Name;id=$_.ID;flags=[string]$_.Flags;hash=(HashBytes $_.Bytes)} })
[IO.File]::WriteAllBytes((Join-Path $out 'graph-original.hkx'),$bnd.Files[3].Bytes)
ConvertHkx (Join-Path $out 'graph-original.hkx')
& python (Join-Path $PSScriptRoot 'behavior_v5.py') patch (Join-Path $out 'graph-original.xml') (Join-Path $out 'graph-patched.xml')
if($LASTEXITCODE -ne 0) {throw 'Graph patch failed'}
ConvertHkx (Join-Path $out 'graph-patched.xml')
Copy-Item -LiteralPath (Join-Path $out 'graph-patched.hkx') -Destination (Join-Path $out 'graph-roundtrip.hkx')
ConvertHkx (Join-Path $out 'graph-roundtrip.hkx')
$patch=[IO.File]::ReadAllBytes((Join-Path $out 'graph-patched.hkx'))
$bnd.Files[3].Bytes=$patch
$target=Join-Path $out 'chr/c0010.behbnd.dcx'
$bnd.Write($target)
$readback=[SoulsFormats.BND4]::Read($target)
if($readback.Files.Count -ne 4) {throw 'Binder count changed'}
for($i=0;$i -lt 4;$i++) {
    $file=$readback.Files[$i];$ref=$before[$i]
    if($file.Name -cne $ref.name -or $file.ID -ne $ref.id -or [string]$file.Flags -cne $ref.flags) {throw 'Binder metadata changed'}
    $expected=if($i -eq 3) {HashBytes $patch} else {$ref.hash}
    if((HashBytes $file.Bytes) -ne $expected) {throw "Binder data mismatch: $i"}
}
& python (Join-Path $PSScriptRoot 'behavior_v5.py') verify (Join-Path $out 'graph-original.xml') (Join-Path $out 'graph-patched.xml') (Join-Path $out 'graph-roundtrip.xml')
if($LASTEXITCODE -ne 0) {throw 'V5 graph verification failed'}
$ani=[SoulsFormats.BND4]::Read((Join-Path $GoldenAgeAssets 'chr/c0010.anibnd.dcx'))
$tae=[SoulsFormats.TAE]::Read([byte[]]@($ani.Files | Where-Object Name -like '*.tae')[0].Bytes)
$notifications=@(foreach($id in @(20010,3035)) {
    $anim=@($tae.Animations | Where-Object ID -eq $id)
    $events=@($anim.Events | Where-Object Type -eq 1011)
    if($events.Count -ne 1) {throw 'Expected one NR revert notification'}
    $event=$events[0]
    $expected=if($id -eq 20010) {17.0/30} else {110.0/30}
    if([Math]::Abs($event.StartTime-$expected) -gt 0.000001 -or [BitConverter]::ToInt32($event.GetParameterBytes($false),0) -ne 0) {throw 'Revert notification changed'}
    @{animation=$id;event=1011;start=$event.StartTime;end=$event.EndTime;revert_type=0}
})
$notifications | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $out 'revert-notifications.json') -Encoding utf8
