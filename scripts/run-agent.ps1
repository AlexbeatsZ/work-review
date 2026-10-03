param(
    [Parameter(Mandatory)][string]$BinaryPath,
    [Parameter(Mandatory)][string]$DataDir
)
$ErrorActionPreference = 'Stop'
$taskArgs = '--data-dir "' + $DataDir + '" run'
$taskProcess = Start-Process -FilePath $BinaryPath -ArgumentList $taskArgs -WindowStyle Hidden -Wait -PassThru `
    -RedirectStandardOutput (Join-Path $DataDir 'agent.stdout.log') `
    -RedirectStandardError (Join-Path $DataDir 'agent.stderr.log')
exit $taskProcess.ExitCode
