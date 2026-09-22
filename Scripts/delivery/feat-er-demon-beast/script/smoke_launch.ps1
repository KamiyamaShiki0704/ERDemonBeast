param(
    [Parameter(Mandatory=$true)][string]$Profile,
    [string]$Loader = 'F:\me3\bin\me3.exe',
    [string]$Game = 'F:\SteamLibrary\steamapps\common\ELDEN RING\Game\eldenring.exe',
    [switch]$SkipSteamInit
)
$ErrorActionPreference = 'Stop'
if (Get-Process eldenring -ErrorAction SilentlyContinue) { throw 'Existing ER process: refusing concurrent launch.' }
$profilePath = (Resolve-Path -LiteralPath $Profile).Path
$outputRoot = Split-Path -Parent $profilePath
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$arguments = @('launch','--profile',('"'+$profilePath+'"'),'--exe',('"'+$Game+'"'),'--online','false','--show-logos','--no-boot-boost','--no-mem-patch')
if ($SkipSteamInit) { $arguments += '--skip-steam-init' }
$launcher = Start-Process -FilePath $Loader -ArgumentList $arguments -WorkingDirectory $outputRoot -WindowStyle Hidden -RedirectStandardOutput "$outputRoot\smoke-$stamp.stdout.log" -RedirectStandardError "$outputRoot\smoke-$stamp.stderr.log" -PassThru
$seen = @()
$gameProcesses = @{}
for ($i=0; $i -lt 30; $i++) {
    Start-Sleep -Milliseconds 500
    $running = @(Get-Process eldenring -ErrorAction SilentlyContinue)
    $seen += @($running | ForEach-Object { $_.Id })
    foreach ($gameProcess in $running) {
        if (-not $gameProcesses.ContainsKey($gameProcess.Id)) {
            $null = $gameProcess.Handle
            $gameProcesses[$gameProcess.Id] = $gameProcess
        }
    }
}
$launcher.Refresh()
$alive = @(Get-Process eldenring -ErrorAction SilentlyContinue | Select-Object Id,MainWindowTitle)
$exits = @($gameProcesses.Values | ForEach-Object { $_.Refresh(); if ($_.HasExited) { @{pid=$_.Id;exit_code=$_.ExitCode} } })
$result = [ordered]@{ timestamp=$stamp; profile=$profilePath; loader=$Loader; skip_steam_init=[bool]$SkipSteamInit; observed_pids=@($seen | Sort-Object -Unique); game_exits=$exits; alive_after_15s=$alive; launcher_exited=$launcher.HasExited; launcher_exit_code=$(if ($launcher.HasExited) {$launcher.ExitCode} else {$null}); status=$(if ($alive.Count) {'PROCESS_ALIVE_NEEDS_UI_CHECK'} else {'FAIL_EARLY_EXIT'}) }
$result | ConvertTo-Json -Depth 5 | Tee-Object -FilePath "$outputRoot\smoke-$stamp.json"
