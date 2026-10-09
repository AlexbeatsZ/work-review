param(
    [string]$BinaryPath,
    [string]$DataDir = (Join-Path $env:LOCALAPPDATA 'work-review-agent'),
    [switch]$Uninstall
)
$ErrorActionPreference = 'Stop'
$taskName = 'WorkReviewAgent-' + $env:USERNAME
$taskData = [IO.Path]::GetFullPath($DataDir)
$taskCopy = Join-Path $taskData 'bin\work-review-agent.exe'
$taskLauncher = Join-Path $taskData 'bin\run-agent.exe'
function Stop-AgentRuntime {
    Stop-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
    # A stopped wrapper can leave its child alive; stop only this installed binary.
    Get-Process -Name work-review-agent -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $taskCopy } | Stop-Process -Force
    Get-Process -Name run-agent -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $taskLauncher } | Stop-Process -Force
}
if ($Uninstall) {
    $task = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
    if ($task) {
        Stop-AgentRuntime
        Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
    }
    Write-Output 'Login task removed; configuration and records preserved.'
    exit
}
if (-not $BinaryPath) { throw 'Provide -BinaryPath pointing to work-review-agent.exe' }
$taskBinary = (Resolve-Path -LiteralPath $BinaryPath).Path
if (-not (Test-Path -LiteralPath (Join-Path $taskData 'config.json'))) { throw 'Run agent init with this data directory first' }
$taskCompiler = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
if (-not (Test-Path -LiteralPath $taskCompiler)) { throw 'The Windows .NET Framework compiler is required for the quiet login launcher.' }
$taskLauncherSource = Join-Path $PSScriptRoot 'run-agent-launcher.cs'
if (-not (Test-Path -LiteralPath $taskLauncherSource)) { throw 'Missing run-agent-launcher.cs beside the installer.' }
$taskUser = [Security.Principal.WindowsIdentity]::GetCurrent().Name
$existingTask = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
if ($existingTask) {
    Stop-AgentRuntime
}
$taskBinDir = Join-Path $taskData 'bin'
New-Item -ItemType Directory -Force -Path $taskBinDir | Out-Null
if ($taskBinary -ne $taskCopy) {
    for ($attempt = 0; ; $attempt++) {
        try { Copy-Item -LiteralPath $taskBinary -Destination $taskCopy -Force; break }
        catch { if ($attempt -ge 9) { throw }; Start-Sleep -Milliseconds 300 }
    }
}
# A GUI-subsystem launcher avoids console delegation before PowerShell can hide it.
& $taskCompiler /nologo /target:winexe /optimize+ ('/out:' + $taskLauncher) $taskLauncherSource
if ($LASTEXITCODE -ne 0) { throw 'Failed to build the quiet login launcher.' }
$taskArguments = '"' + $taskData.TrimEnd([char]'\') + '"'
$taskAction = New-ScheduledTaskAction -Execute $taskLauncher -Argument $taskArguments -WorkingDirectory $taskData
$taskTrigger = New-ScheduledTaskTrigger -AtLogOn -User $taskUser
$taskPrincipal = New-ScheduledTaskPrincipal -UserId $taskUser -LogonType Interactive -RunLevel Limited
$taskSettings = New-ScheduledTaskSettingsSet -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew -StartWhenAvailable -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName $taskName -Action $taskAction -Trigger $taskTrigger -Principal $taskPrincipal -Settings $taskSettings -Force | Out-Null
Start-ScheduledTask -TaskName $taskName
Write-Output ('Background collector started. Logs: ' + $taskData)
