param(
    [ValidateSet('start','end','mark')][string]$Action = 'start',
    [ValidateRange(1,6)][int]$Case = 1,
    [string]$Note = ''
)
$ErrorActionPreference = 'Stop'
$latest = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'manual-latest.json') -Raw | ConvertFrom-Json
$arguments = @('command',$latest.run,$Action)
if ($Action -eq 'start') { $arguments += @('--case',$Case) }
if ($Note) { $arguments += @('--note',$Note) }
& python (Join-Path $PSScriptRoot 'probe.py') @arguments
if ($LASTEXITCODE -ne 0) { throw 'Probe marker was not sent.' }
