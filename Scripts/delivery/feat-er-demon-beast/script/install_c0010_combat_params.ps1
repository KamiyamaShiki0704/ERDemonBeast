param(
    [Parameter(Mandatory=$true)][string]$Report,
    [string]$BackupRoot='F:\GoldenAge\_GoldenAge\dll\demon_beast\_backups',
    [switch]$VerifyOnly
)
$ErrorActionPreference='Stop'
function Hash([string]$path) {(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()}
function GameExitGuard {if(Get-Process -Name eldenring -ErrorAction SilentlyContinue) {throw 'Exit Elden Ring before installing parameters'}}
$record=Get-Content -LiteralPath $Report -Raw | ConvertFrom-Json
if($record.kind -ne 'c0010-combat-param-port' -or $record.spec -ne 'spec-combat-params.md@1.0' -or $record.verification -ne 'PASS') {throw 'A verified combat parameter package is required'}
if((Hash $record.candidate) -ne $record.candidate_sha256) {throw 'Candidate hash differs from verified package'}
if((Hash $record.source) -ne $record.source_sha256) {throw 'Current regulation changed after preparation; regenerate the merge'}
if((Hash $record.animations) -ne $record.animations_sha256) {throw 'Animation input changed after reference validation'}
if($VerifyOnly) {Write-Output 'PASS: candidate and installed inputs match verified report'; exit 0}
GameExitGuard
$backup=Join-Path ([IO.Path]::GetFullPath($BackupRoot)) ('regulation-combat-'+(Get-Date -Format 'yyyyMMdd-HHmmss-fff')+'-'+[guid]::NewGuid().ToString('N').Substring(0,8))
New-Item -ItemType Directory -Path $backup | Out-Null
$original=Join-Path $backup 'regulation.bin'
Copy-Item -LiteralPath $record.source -Destination $original
if((Hash $original) -ne $record.source_sha256) {throw 'Backup differs from verified input'}
Copy-Item -LiteralPath $Report -Destination (Join-Path $backup 'port-report.json')
$restore=Join-Path $backup 'Restore-Regulation.ps1'
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'restore_regulation_repair.ps1') -Destination $restore
$deployment=Join-Path $backup 'deployment.json'
$result=[ordered]@{
    status='prepared';kind=$record.kind;target=$record.source;before=$record.source_sha256;after=$record.candidate_sha256
    backup=$original;candidate=$record.candidate;report_sha256=(Hash $Report);added_count=$record.added_count
    preserved_existing_rows=$record.preserved_existing_rows;unchanged_tables=$record.unchanged_tables
    runtime_tested=$false;restore_script=$restore;timestamp=(Get-Date -Format o)
}
$result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $deployment -Encoding utf8
$temp=Join-Path (Split-Path -Parent $record.source) ('regulation-combat-'+[guid]::NewGuid().ToString('N')+'.tmp')
Copy-Item -LiteralPath $record.candidate -Destination $temp
if((Hash $temp) -ne $record.candidate_sha256) {throw 'Staged candidate hash mismatch'}
GameExitGuard
if((Hash $record.source) -ne $record.source_sha256) {throw 'Current regulation changed while staging'}
$atomicBackup=Join-Path $backup 'atomic-original.bin'
[IO.File]::Replace($temp,[string]$record.source,$atomicBackup)
if((Hash $record.source) -ne $record.candidate_sha256 -or (Hash $atomicBackup) -ne $record.source_sha256) {throw 'Installed hash mismatch; original and replacement files are retained'}
$result.status='installed'
$result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $deployment -Encoding utf8
$launcher='@echo off'+"`r`n"+'pwsh -NoProfile -ExecutionPolicy Bypass -File "%~dp0Restore-Regulation.ps1" -Deployment "%~dp0deployment.json"'+"`r`n"+'pause'+"`r`n"
[IO.File]::WriteAllText((Join-Path $backup 'Restore-Regulation.cmd'),$launcher,[Text.Encoding]::ASCII)
$result | ConvertTo-Json -Depth 8
Write-Output "Deployment record: $deployment"
