param(
    [string]$Deployment='F:\GoldenAge\_GoldenAge\dll\demon_beast\_backups\regulation-repair-20260925-192733-997\deployment.json',
    [switch]$VerifyOnly
)
$ErrorActionPreference='Stop'
function Hash([string]$path) {(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()}
$record=Get-Content -LiteralPath $Deployment -Raw | ConvertFrom-Json
if($record.status -ne 'installed') {throw 'Deployment is not marked installed'}
if((Hash $record.backup) -ne $record.before) {throw 'Backup hash differs from recorded original'}
if((Hash $record.target) -ne $record.after) {throw 'Current regulation has later changes; refusing to overwrite them'}
if($VerifyOnly) {Write-Output 'PASS: backup and current regulation match the recorded repair'; exit 0}
if(Get-Process -Name eldenring -ErrorAction SilentlyContinue) {throw 'Exit Elden Ring before restoring regulation'}
$temp=Join-Path (Split-Path -Parent $record.target) ('regulation-repair-restore-'+[guid]::NewGuid().ToString('N')+'.tmp')
$replaced=Join-Path (Split-Path -Parent $Deployment) ('before-restore-'+[guid]::NewGuid().ToString('N')+'.bin')
Copy-Item -LiteralPath $record.backup -Destination $temp
if((Hash $temp) -ne $record.before) {throw 'Restore staging hash mismatch'}
if(Get-Process -Name eldenring -ErrorAction SilentlyContinue) {throw 'Elden Ring started while staging; restore refused'}
if((Hash $record.target) -ne $record.after) {throw 'Current regulation changed while staging; restore refused'}
# A real backup path is required: PowerShell coerces a null string to an empty path.
[IO.File]::Replace($temp,[string]$record.target,$replaced)
if((Hash $record.target) -ne $record.before -or (Hash $replaced) -ne $record.after) {throw 'Restore verification failed; inspect both retained files'}
Write-Output "Restored original regulation. Repaired version retained at $replaced"
