param(
    [ValidateSet('Install','Restore')][string]$Mode='Restore',
    [string]$Package,
    [string]$GaRoot='F:/GoldenAge/_GoldenAge/GA',
    [string]$DllRoot='F:/GoldenAge/_GoldenAge/dll/demon_beast',
    [string]$Record
)
# Hash-guarded three-asset transaction. Roots may point at disposable offline fixtures.
$ErrorActionPreference='Stop'
if (Get-Process eldenring -ErrorAction SilentlyContinue) { throw 'Close ER before installing or restoring.' }
$GaRoot=[IO.Path]::GetFullPath($GaRoot)
$DllRoot=[IO.Path]::GetFullPath($DllRoot)
if (-not $Record) { $Record=Join-Path $DllRoot 'deployment-split-hks-1.json' }
$names=@('chr/c0010.behbnd.dcx','action/script/c9997.hks','action/script/d0010.hks')
$protectedNames=@('regulation.bin','chr/c0010.anibnd.dcx','action/script/c0010.hks','action/eventnameid.txt','action/statenameid.txt','action/variablenameid.txt')
$dllNames=@('er_demon_beast.dll','er-demon-beast.toml')
function Hash([string]$path) { if (Test-Path -LiteralPath $path -PathType Leaf) { (Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant() } else { $null } }
function SameSet($actual,$expected) { (($actual | Sort-Object) -join ',') -ceq (($expected | Sort-Object) -join ',') }
function Target($file) {
    if ($file.relative -cnotin $names) { throw 'Unknown transaction target' }
    Join-Path $GaRoot $file.relative
}
function ReplaceFile([string]$source,[string]$target) {
    $temp=$target+'.erdb-'+[Guid]::NewGuid().ToString('N')
    Copy-Item -LiteralPath $source -Destination $temp
    $old=$temp+'.old'
    [IO.File]::Replace($temp,$target,$old)
    Remove-Item -LiteralPath $old
}
function CheckProtected($data) {
    if (-not (SameSet @($data.protected.relative) $protectedNames)) { throw 'Protected GA set mismatch' }
    if (-not (SameSet @($data.dll_protected.relative) $dllNames)) { throw 'Protected DLL set mismatch' }
    foreach ($p in $data.protected) {
        if ((Hash (Join-Path $GaRoot $p.relative)) -ne $p.hash) { throw "Protected GA file changed: $($p.relative)" }
    }
    foreach ($p in $data.dll_protected) {
        if ((Hash (Join-Path $DllRoot $p.relative)) -ne $p.hash) { throw "Protected DLL file changed: $($p.relative)" }
    }
}
if ($Mode -eq 'Install') {
    if (-not $Package -or (Test-Path -LiteralPath $Record)) { throw 'Fresh record and package required' }
    $data=Get-Content -LiteralPath (Join-Path $Package 'split-build.json') -Raw | ConvertFrom-Json
    $validation=Get-Content -LiteralPath (Join-Path $Package 'split-verification.json') -Raw | ConvertFrom-Json
    if ($data.result -ne 'PASS_ASSET_STRUCTURE' -or $validation.result -ne 'PASS_OFFLINE') { throw 'Package has not passed offline checks' }
    if (-not (SameSet @($data.files.relative) $names)) { throw 'Transaction file set mismatch' }
    CheckProtected $data
    foreach ($file in $data.files) {
        if ((Hash (Target $file)) -ne $file.before) { throw "Installed source changed: $($file.relative)" }
        if ((Hash (Join-Path $Package $file.relative)) -ne $file.after -or $validation.files.($file.relative) -ne $file.after) { throw 'Package or verification hash mismatch' }
    }
    $backup=Join-Path $DllRoot ('_backups/split-hks-'+(Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
    foreach ($file in $data.files) {
        $dest=Join-Path $backup $file.relative
        New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
        Copy-Item -LiteralPath (Target $file) -Destination $dest
        if ((Hash $dest) -ne $file.before) { throw 'Backup verification failed' }
    }
    $data | Add-Member backup $backup
    $data | Add-Member ga_root $GaRoot
    $data | Add-Member dll_root $DllRoot
    $data | Add-Member status 'installing'
    $data | Add-Member verification_sha256 (Hash (Join-Path $Package 'split-verification.json'))
    $data | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Record -Encoding utf8
    $sourceRoot=$Package; $expectedField='after'; $priorField='before'
} else {
    $data=Get-Content -LiteralPath $Record -Raw | ConvertFrom-Json
    if ($data.status -ne 'installed' -or $data.ga_root -ne $GaRoot -or $data.dll_root -ne $DllRoot) { throw 'Installed transaction for these roots required' }
    if (-not (SameSet @($data.files.relative) $names)) { throw 'Transaction file set mismatch' }
    CheckProtected $data
    foreach ($file in $data.files) {
        if ((Hash (Target $file)) -ne $file.after) { throw "Later edit detected: $($file.relative)" }
        if ((Hash (Join-Path $data.backup $file.relative)) -ne $file.before) { throw 'Backup hash mismatch' }
    }
    # Preserve the installed set too, so a failed restore can roll forward as a unit.
    $recovery=Join-Path $data.backup ('restore-recovery-'+[Guid]::NewGuid().ToString('N'))
    foreach ($file in $data.files) {
        $dest=Join-Path $recovery $file.relative
        New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
        Copy-Item -LiteralPath (Target $file) -Destination $dest
        if ((Hash $dest) -ne $file.after) { throw 'Restore recovery hash mismatch' }
    }
    $sourceRoot=$data.backup; $expectedField='before'; $priorField='after'
}
$applied=[Collections.Generic.List[object]]::new()
try {
    foreach ($file in $data.files) {
        if (Get-Process eldenring -ErrorAction SilentlyContinue) { throw 'ER started during transaction' }
        if ((Hash (Target $file)) -ne $file.$priorField) { throw 'Concurrent target edit' }
        if ((Hash (Join-Path $sourceRoot $file.relative)) -ne $file.$expectedField) { throw 'Concurrent source edit' }
        ReplaceFile (Join-Path $sourceRoot $file.relative) (Target $file)
        $applied.Add($file)
        if ((Hash (Target $file)) -ne $file.$expectedField) { throw 'Replacement hash mismatch' }
    }
    CheckProtected $data
    $data.status=if ($Mode -eq 'Install') { 'installed' } else { 'restored' }
} catch {
    $failure=$_
    for ($i=$applied.Count-1; $i -ge 0; $i--) {
        $file=$applied[$i]
        if ((Hash (Target $file)) -ne $file.$expectedField) { throw 'Concurrent edit prevents recovery; retain transaction record and backups' }
        $recoveryRoot=if ($Mode -eq 'Install') { $data.backup } else { $recovery }
        ReplaceFile (Join-Path $recoveryRoot $file.relative) (Target $file)
        if ((Hash (Target $file)) -ne $file.$priorField) { throw 'Recovery hash mismatch; retain backups' }
    }
    $data.status=if ($Mode -eq 'Install') { 'rolled_back' } else { 'installed' }
    $data | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Record -Encoding utf8
    throw $failure
}
$data | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Record -Encoding utf8
Write-Output "$($data.status): three behavior assets; backup=$($data.backup)"
