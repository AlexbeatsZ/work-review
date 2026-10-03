param(
    [string]$BinaryPath,
    [Parameter(Mandatory)][string]$DataDir,
    [switch]$Uninstall
)
$ErrorActionPreference = 'Stop'
$taskName = 'WorkReviewHub'
$hubData = [IO.Path]::GetFullPath($DataDir)
$hubCopy = Join-Path $hubData 'bin\work-review-server.exe'
function Stop-HubRuntime {
    Stop-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
    # A stopped wrapper can leave its child alive; stop only this installed binary.
    Get-Process -Name work-review-server -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $hubCopy } | Stop-Process -Force
}
$hubIdentity = [Security.Principal.WindowsIdentity]::GetCurrent()
$hubPrincipal = [Security.Principal.WindowsPrincipal]::new($hubIdentity)
if (-not $hubPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run in an elevated PowerShell session to register the startup task.'
}
if ($Uninstall) {
    if (Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue) {
        Stop-HubRuntime
        Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
    }
    Write-Output 'Hub startup task removed; configuration and records preserved.'
    exit
}
if (-not $BinaryPath) { throw 'Provide -BinaryPath pointing to work-review-server.exe' }
$hubBinary = (Resolve-Path -LiteralPath $BinaryPath).Path
if (-not (Test-Path -LiteralPath (Join-Path $hubData 'config.json'))) { throw 'Run server init with this data directory first.' }
if (Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue) {
    Stop-HubRuntime
}
$hubBinDir = Join-Path $hubData 'bin'
New-Item -ItemType Directory -Force -Path $hubBinDir | Out-Null
if ($hubBinary -ne $hubCopy) {
    for ($attempt = 0; ; $attempt++) {
        try { Copy-Item -LiteralPath $hubBinary -Destination $hubCopy -Force; break }
        catch { if ($attempt -ge 9) { throw }; Start-Sleep -Milliseconds 300 }
    }
}
$hubRunner = Join-Path $hubBinDir 'run-hub.ps1'
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-hub.ps1') -Destination $hubRunner -Force
$hubShell = Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe'
$hubArguments = '-NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File "' + $hubRunner + '" -BinaryPath "' + $hubCopy + '" -DataDir "' + $hubData + '"'
$hubAction = New-ScheduledTaskAction -Execute $hubShell -Argument $hubArguments -WorkingDirectory $hubData
$hubTrigger = New-ScheduledTaskTrigger -AtStartup
$hubTaskPrincipal = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -LogonType ServiceAccount
$hubSettings = New-ScheduledTaskSettingsSet -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew -StartWhenAvailable -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName $taskName -Action $hubAction -Trigger $hubTrigger -Principal $hubTaskPrincipal -Settings $hubSettings -Force | Out-Null
Start-ScheduledTask -TaskName $taskName
Write-Output ('Hub startup task installed. Logs: ' + $hubData)
