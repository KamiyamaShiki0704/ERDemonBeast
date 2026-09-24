param(
    [ValidateSet('Install','Restore')][string]$Mode='Restore',
    [string]$Package,
    [string]$Record=(Join-Path $PSScriptRoot 'deployment-forms-0.4.json')
)
# One transaction across native DLL, behavior assets and metadata. No game launch.
$ErrorActionPreference='Stop'
if(Get-Process eldenring -ErrorAction SilentlyContinue) {throw 'Close ER before installing or restoring.'}
$dllRoot='F:\GoldenAge\_GoldenAge\dll\demon_beast'
$gaRoot='F:\GoldenAge\_GoldenAge\GA'
$assetNames=@('action/script/d0010.hks','chr/c0010.behbnd.dcx')
$dllNames=@('er_demon_beast.dll','er_demon_beast.pdb','er-demon-beast.toml','build-result.json','behavior-build.json','behavior-deployment.json','BEHAVIOR_TEST.md','probe.py','manual_test.py')
function Hash([string]$path) {if(Test-Path -LiteralPath $path) {(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()} else {$null}}
function Target($file) {
    if($file.scope -eq 'ga' -and $file.relative -cin $assetNames) {$root=$gaRoot}
    elseif($file.scope -eq 'dll' -and $file.relative -cin $dllNames) {$root=$dllRoot}
    else {throw 'Unapproved transaction target'}
    $path=[IO.Path]::GetFullPath((Join-Path $root $file.relative))
    if(-not $path.StartsWith($root+'\',[StringComparison]::OrdinalIgnoreCase)) {throw 'Target escaped root'}
    $path
}
function ReplaceFile([string]$source,[string]$target) {
    $temp=$target+'.erdb-'+[Guid]::NewGuid().ToString('N')
    Copy-Item -LiteralPath $source -Destination $temp
    if(Test-Path -LiteralPath $target) {
        $old=$temp+'.old';[IO.File]::Replace($temp,$target,$old);Remove-Item -LiteralPath $old
    } else {Move-Item -LiteralPath $temp -Destination $target}
}
if($Mode -eq 'Restore') {
    $data=Get-Content -LiteralPath $Record -Raw | ConvertFrom-Json
    if($data.status -ne 'installed') {throw 'No installed transaction'}
    foreach($file in $data.files) {
        if((Hash (Target $file)) -ne $file.after) {throw "Later edit detected: $($file.relative)"}
        if((Hash (Join-Path $data.backup ($file.scope+'/'+$file.relative))) -ne $file.before) {throw 'Backup hash mismatch'}
    }
    foreach($file in $data.files) {
        ReplaceFile (Join-Path $data.backup ($file.scope+'/'+$file.relative)) (Target $file)
        if((Hash (Target $file)) -ne $file.before) {throw 'Restore hash mismatch'}
    }
    $data.status='restored';$data | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Record -Encoding utf8
    Write-Output 'RESTORED: all files to the exact pre-install backup recorded in this transaction.'
    return
}
if(-not $Package -or (Test-Path -LiteralPath $Record)) {throw 'Fresh transaction record and package required'}
$manifest=Get-Content -LiteralPath (Join-Path $Package 'upgrade-manifest.json') -Raw | ConvertFrom-Json
if($manifest.release_ready -ne $true) {throw 'Package is not ready for installation; inspect its verification and requirement records.'}
$expected=@($assetNames | ForEach-Object {'ga/'+$_})+@($dllNames | ForEach-Object {'dll/'+$_})
if((($manifest.files | ForEach-Object {$_.scope+'/'+$_.relative} | Sort-Object) -join ',') -cne (($expected | Sort-Object) -join ',')) {throw 'Transaction file set mismatch'}
foreach($file in $manifest.files) {
    if((Hash (Target $file)) -ne $file.before) {throw "Installed source changed: $($file.relative)"}
    if((Hash (Join-Path $Package ($file.scope+'/'+$file.relative))) -ne $file.after) {throw 'Package hash mismatch'}
}
foreach($item in $manifest.protected_inputs.PSObject.Properties) {
    if($item.Name -cnotin @('action/script/c9997.hks','action/script/c0010.hks','chr/c0010.anibnd.dcx','regulation.bin','action/eventnameid.txt','action/statenameid.txt','action/variablenameid.txt')) {throw 'Unknown protected input'}
    if((Hash (Join-Path $gaRoot $item.Name)) -ne $item.Value) {throw "Protected input changed: $($item.Name)"}
}
$backup=Join-Path $dllRoot ('_backups/complete-forms-'+(Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
foreach($file in $manifest.files) {
    $destination=Join-Path $backup ($file.scope+'/'+$file.relative)
    New-Item -ItemType Directory -Force -Path (Split-Path $destination) | Out-Null
    Copy-Item -LiteralPath (Target $file) -Destination $destination
    if((Hash $destination) -ne $file.before) {throw 'Backup verification failed'}
}
$data=[ordered]@{status='installing';backup=$backup;files=$manifest.files;protected_inputs=$manifest.protected_inputs}
$data | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Record -Encoding utf8
$applied=[Collections.Generic.List[object]]::new()
try {
    foreach($file in $manifest.files) {
        ReplaceFile (Join-Path $Package ($file.scope+'/'+$file.relative)) (Target $file)
        $applied.Add($file)
        if((Hash (Target $file)) -ne $file.after) {throw 'Install hash mismatch'}
    }
    $data.status='installed'
} catch {
    $failure=$_
    for($i=$applied.Count-1;$i -ge 0;$i--) {
        $file=$applied[$i]
        if((Hash (Target $file)) -ne $file.after) {throw 'Concurrent edit prevented automatic rollback; retain record'}
        ReplaceFile (Join-Path $backup ($file.scope+'/'+$file.relative)) (Target $file)
    }
    $data.status='rolled_back'
    $data | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Record -Encoding utf8
    throw $failure
}
$data | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Record -Encoding utf8
Write-Output "INSTALLED: paired DLL/behavior package; backup=$backup"
