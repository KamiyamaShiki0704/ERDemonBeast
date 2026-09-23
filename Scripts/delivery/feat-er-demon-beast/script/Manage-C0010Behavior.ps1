param(
    [ValidateSet('Install','Restore')][string]$Mode = 'Install',
    [string]$Package = (Join-Path $PSScriptRoot 'behavior-v1'),
    [string]$GameAssets = 'F:/GoldenAge/_GoldenAge/GA',
    [string]$Record = (Join-Path $PSScriptRoot 'behavior-deployment.json')
)
$ErrorActionPreference = 'Stop'
if(Get-Process eldenring -ErrorAction SilentlyContinue) { throw 'Close ER completely before changing behavior assets.' }
$root = [IO.Path]::GetFullPath($GameAssets).TrimEnd('\','/')
$allowed = @('action/eventnameid.txt','action/statenameid.txt','action/variablenameid.txt','action/script/d0010.hks','chr/c0010.behbnd.dcx')
function Target([string]$relative) {
    if($relative -cnotin $allowed) { throw "Unapproved target: $relative" }
    $path = [IO.Path]::GetFullPath((Join-Path $root $relative))
    if(-not $path.StartsWith($root + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)) { throw 'Target escaped asset root' }
    return $path
}
function Hash([string]$path) {
    if(-not (Test-Path -LiteralPath $path)) { return $null }
    return (Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()
}
# PowerShell can coerce null File.Replace backup paths to empty strings; use a
# dedicated private sibling backup, then remove exactly that temporary file.
function ReplaceFile([string]$source,[string]$target) {
    New-Item -ItemType Directory -Force -Path (Split-Path $target) | Out-Null
    $token = [Guid]::NewGuid().ToString('N')
    $stage = $target + '.erdb-' + $token
    Copy-Item -LiteralPath $source -Destination $stage
    if(Test-Path -LiteralPath $target) {
        $old = $target + '.erdb-old-' + $token
        [IO.File]::Replace($stage,$target,$old)
        Remove-Item -LiteralPath $old
    } else { Move-Item -LiteralPath $stage -Destination $target }
}
if($Mode -eq 'Restore') {
    $installed = Get-Content -LiteralPath $Record -Raw | ConvertFrom-Json
    if($installed.root -cne $root -or $installed.status -ne 'installed') { throw 'Record does not describe this installed asset root' }
    foreach($file in $installed.files) {
        $target = Target $file.relative
        if((Hash $target) -ne $file.after) { throw "Refusing to replace later edits: $($file.relative)" }
        if($null -ne $file.before -and (Hash (Join-Path $installed.backup $file.relative)) -ne $file.before) { throw 'Backup hash mismatch' }
    }
    foreach($file in $installed.files) {
        $target = Target $file.relative
        if($null -eq $file.before) { Remove-Item -LiteralPath $target }
        else { ReplaceFile (Join-Path $installed.backup $file.relative) $target }
        if((Hash $target) -ne $file.before) { throw 'Restore hash mismatch' }
    }
    $installed.status = 'restored'
    $installed | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Record -Encoding utf8
    Write-Output 'RESTORED: all five files match their pre-install state.'
    return
}
$manifest = Get-Content -LiteralPath (Join-Path $Package 'behavior-build.json') -Raw | ConvertFrom-Json
if((($manifest.files.relative | Sort-Object) -join ',') -cne (($allowed | Sort-Object) -join ',')) { throw 'Package file set mismatch' }
foreach($file in $manifest.files) {
    $target = Target $file.relative
    if((Hash $target) -ne $file.before) { throw "Installed source changed: $($file.relative)" }
    if((Hash (Join-Path $Package $file.relative)) -ne $file.after) { throw 'Package hash mismatch' }
}
foreach($entry in $manifest.protected_inputs.PSObject.Properties) {
    if($entry.Name -cnotin @('action/script/c9997.hks','action/script/c0010.hks','chr/c0010.anibnd.dcx','regulation.bin')) { throw 'Unknown protected input' }
    if((Hash (Join-Path $root $entry.Name)) -ne $entry.Value) { throw "Protected dependency changed: $($entry.Name)" }
}
if(Test-Path -LiteralPath $Record) { throw 'Deployment record exists; archive or restore it before a new installation.' }
$backup = Join-Path (Split-Path ([IO.Path]::GetFullPath($Record))) ('_backups/behavior-' + (Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
New-Item -ItemType Directory -Force -Path $backup | Out-Null
foreach($file in $manifest.files) {
    if($null -ne $file.before) {
        $destination=Join-Path $backup $file.relative
        New-Item -ItemType Directory -Force -Path (Split-Path $destination) | Out-Null
        Copy-Item -LiteralPath (Target $file.relative) -Destination $destination
        if((Hash $destination) -ne $file.before) { throw 'Backup verification failed' }
    }
}
$recordData = [ordered]@{root=$root;backup=$backup;status='installing';files=$manifest.files;protected_inputs=$manifest.protected_inputs}
$recordData | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Record -Encoding utf8
$applied = [Collections.Generic.List[object]]::new()
try {
    foreach($file in $manifest.files) {
        ReplaceFile (Join-Path $Package $file.relative) (Target $file.relative)
        $applied.Add($file)
        if((Hash (Target $file.relative)) -ne $file.after) { throw 'Installed hash mismatch' }
    }
    $recordData.status = 'installed'
    $recordData | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Record -Encoding utf8
} catch {
    $failure = $_
    for($i=$applied.Count-1; $i -ge 0; $i--) {
        $file=$applied[$i]; $target=Target $file.relative
        if((Hash $target) -ne $file.after) { throw 'Install failed and rollback refused a concurrent edit; retain deployment record.' }
        if($null -eq $file.before) { Remove-Item -LiteralPath $target }
        else { ReplaceFile (Join-Path $backup $file.relative) $target }
    }
    $recordData.status = 'rolled_back'
    $recordData | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Record -Encoding utf8
    throw $failure
}
Write-Output "INSTALLED: verified five files; backup=$backup"
