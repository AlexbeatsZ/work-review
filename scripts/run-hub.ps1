param(
    [Parameter(Mandatory)][string]$BinaryPath,
    [Parameter(Mandatory)][string]$DataDir
)
$ErrorActionPreference = 'Stop'
$hubArguments = '--data-dir "' + $DataDir + '" run'
$hubProcess = Start-Process -FilePath $BinaryPath -ArgumentList $hubArguments -WindowStyle Hidden -Wait -PassThru `
    -WorkingDirectory $DataDir `
    -RedirectStandardOutput (Join-Path $DataDir 'hub.stdout.log') `
    -RedirectStandardError (Join-Path $DataDir 'hub.stderr.log')
exit $hubProcess.ExitCode
