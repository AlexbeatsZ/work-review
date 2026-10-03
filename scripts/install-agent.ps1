param(
    [string]$BinaryPath,
    [string]$DataDir = (Join-Path $env:LOCALAPPDATA 'work-review-agent'),
    [switch]$Uninstall
)
$ErrorActionPreference = 'Stop'
$taskName = 'WorkReviewAgent-' + $env:USERNAME
if ($Uninstall) {
    $task = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
    if ($task) {
        Stop-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
        Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
    }
    Write-Output 'Login task removed; configuration and records preserved.'
    exit
}
if (-not $BinaryPath) { throw 'Provide -BinaryPath pointing to work-review-agent.exe' }
$taskBinary = (Resolve-Path -LiteralPath $BinaryPath).Path
$taskData = [IO.Path]::GetFullPath($DataDir)
if (-not (Test-Path -LiteralPath (Join-Path $taskData 'config.json'))) { throw 'Run agent init with this data directory first' }
$taskUser = [Security.Principal.WindowsIdentity]::GetCurrent().Name
$existingTask = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
if ($existingTask) {
    Stop-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
}
$taskBinDir = Join-Path $taskData 'bin'
New-Item -ItemType Directory -Force -Path $taskBinDir | Out-Null
$taskCopy = Join-Path $taskBinDir 'work-review-agent.exe'
if ($taskBinary -ne $taskCopy) {
    for ($attempt = 0; ; $attempt++) {
        try { Copy-Item -LiteralPath $taskBinary -Destination $taskCopy -Force; break }
        catch { if ($attempt -ge 9) { throw }; Start-Sleep -Milliseconds 300 }
    }
}
$taskRunner = Join-Path $taskBinDir 'run-agent.ps1'
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-agent.ps1') -Destination $taskRunner -Force
$taskArguments = '-NoProfile -NonInteractive -WindowStyle Hidden -File "' + $taskRunner + '" -BinaryPath "' + $taskCopy + '" -DataDir "' + $taskData + '"'
$taskAction = New-ScheduledTaskAction -Execute (Join-Path $PSHOME 'powershell.exe') -Argument $taskArguments
if (-not (Test-Path -LiteralPath $taskAction.Execute)) {
    $taskAction = New-ScheduledTaskAction -Execute (Join-Path $PSHOME 'pwsh.exe') -Argument $taskArguments
}
$taskTrigger = New-ScheduledTaskTrigger -AtLogOn -User $taskUser
$taskPrincipal = New-ScheduledTaskPrincipal -UserId $taskUser -LogonType Interactive -RunLevel Limited
$taskSettings = New-ScheduledTaskSettingsSet -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew -StartWhenAvailable -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName $taskName -Action $taskAction -Trigger $taskTrigger -Principal $taskPrincipal -Settings $taskSettings -Force | Out-Null
Start-ScheduledTask -TaskName $taskName
Write-Output ('Background collector started. Logs: ' + $taskData)
