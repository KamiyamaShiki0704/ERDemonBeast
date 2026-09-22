param(
    [ValidateSet('load','observe','control','missing')][string]$Mode = 'load',
    [switch]$FullStack,
    [switch]$PrepareOnly,
    [string]$GoldenAge = 'F:\GoldenAge\_GoldenAge',
    [string]$Game = 'F:\SteamLibrary\steamapps\common\ELDEN RING\Game\eldenring.exe'
)
$ErrorActionPreference = 'Stop'
if (Get-Process eldenring -ErrorAction SilentlyContinue) { throw 'Close ER completely before starting another test.' }
$root = $PSScriptRoot
$loader = Join-Path $GoldenAge 'me3\bin\me3.exe'
if (-not (Test-Path -LiteralPath $loader) -or -not (Test-Path -LiteralPath $Game)) { throw 'Missing ME3 or ER executable.' }
$arguments = @('prepare',$root,'--ga',$GoldenAge,'--mode',$Mode,'--save-root',("$env:APPDATA\EldenRing"))
if ($FullStack) { $arguments += '--full-stack' }
$prepared = & python (Join-Path $root 'manual_test.py') @arguments
if ($LASTEXITCODE -ne 0) { throw 'Preparation failed; no game was started.' }
$info = $prepared | ConvertFrom-Json
Write-Host ('Mode: '+$Mode+'; full stack: '+$FullStack)
Write-Host ('Logs: '+$info.run)
Write-Host ('Save backup: '+$info.save_backup)
if ($PrepareOnly) { return }
# Only the user executes this launcher. Online is disabled regardless of base profile.
$launchArguments = @('launch','--profile',('"'+$info.profile+'"'),'--exe',('"'+$Game+'"'),'--skip-steam-init','--online','false','--show-logos','--no-boot-boost','--no-mem-patch')
$process = Start-Process -FilePath $loader -ArgumentList $launchArguments -WindowStyle Hidden -PassThru -Wait -RedirectStandardOutput (Join-Path $info.run 'launcher.stdout.log') -RedirectStandardError (Join-Path $info.run 'launcher.stderr.log')
if ($process.ExitCode -ne 0) { throw "ME3 exited with $($process.ExitCode); retain launcher logs" }
