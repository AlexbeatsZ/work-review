$ErrorActionPreference = 'Stop'
$compiler = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
$tempBase = [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) '.agents'))
$testRoot = Join-Path $tempBase ('work-review-launcher-' + [Guid]::NewGuid().ToString('N'))
$testData = Join-Path $testRoot ('data with spaces ' + [char]0x6D4B + [char]0x8BD5)
$testBin = Join-Path $testRoot 'bin'
function Assert-Launcher($condition, [string]$message) {
    if (-not $condition) { throw $message }
}
function Invoke-TestLauncher {
    $start = [Diagnostics.ProcessStartInfo]::new((Join-Path $testBin 'run-agent.exe'), ('"' + $testData + '"'))
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $process = [Diagnostics.Process]::Start($start)
    try {
        if (-not $process.WaitForExit(10000)) { $process.Kill(); throw 'Launcher timed out' }
        return $process.ExitCode
    } finally { $process.Dispose() }
}
try {
    New-Item -ItemType Directory -Path $testData,$testBin -Force | Out-Null
    $workerSource = Join-Path $testRoot 'worker.cs'
    @'
using System;
using System.Runtime.InteropServices;
using System.Text;
internal static class Worker {
    [DllImport("kernel32.dll")] private static extern IntPtr GetConsoleWindow();
    private static int Main(string[] args) {
        Console.WriteLine("console=" + GetConsoleWindow().ToInt64());
        foreach (string arg in args)
            Console.WriteLine("arg=" + Convert.ToBase64String(Encoding.UTF8.GetBytes(arg)));
        Console.Error.WriteLine("worker stderr");
        return 23;
    }
}
'@ | Set-Content -LiteralPath $workerSource -Encoding Ascii
    & $compiler /nologo /target:exe ('/out:' + (Join-Path $testBin 'work-review-agent.exe')) $workerSource
    Assert-Launcher ($LASTEXITCODE -eq 0) 'Fixture compiler failed'
    & $compiler /nologo /target:winexe /optimize+ ('/out:' + (Join-Path $testBin 'run-agent.exe')) (Join-Path $PSScriptRoot 'run-agent-launcher.cs')
    Assert-Launcher ($LASTEXITCODE -eq 0) 'Launcher compiler failed'
    $stdoutPath = Join-Path $testData 'agent.stdout.log'
    $stderrPath = Join-Path $testData 'agent.stderr.log'
    'prior stdout' | Set-Content -LiteralPath $stdoutPath -Encoding Ascii
    'prior stderr' | Set-Content -LiteralPath $stderrPath -Encoding Ascii
    Assert-Launcher ((Invoke-TestLauncher) -eq 23) 'Child exit code was lost'
    $outputLines = [IO.File]::ReadAllLines($stdoutPath)
    Assert-Launcher ($outputLines[0] -eq 'prior stdout') 'Old stdout log was overwritten'
    Assert-Launcher ($outputLines -contains 'console=0') 'Console-subsystem child allocated a console'
    $actualArgs = @($outputLines | Where-Object { $_.StartsWith('arg=') } | ForEach-Object {
        [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($_.Substring(4)))
    })
    Assert-Launcher ($actualArgs.Count -eq 3 -and $actualArgs[0] -eq '--data-dir' -and $actualArgs[1] -eq $testData -and $actualArgs[2] -eq 'run') 'Spaced or Unicode data directory arguments changed'
    $errorText = [IO.File]::ReadAllText($stderrPath)
    Assert-Launcher ($errorText.Contains('prior stderr') -and $errorText.Contains('worker stderr')) 'Stderr was lost or overwritten'
    Remove-Item -LiteralPath (Join-Path $testBin 'work-review-agent.exe')
    Assert-Launcher ((Invoke-TestLauncher) -eq 1) 'Missing binary must fail for Task Scheduler retries'
    Assert-Launcher ([IO.File]::ReadAllText($stderrPath).Contains('launcher:')) 'Startup failure was not logged'
    Write-Output 'Launcher checks passed: no console, arguments, log append, exit code, startup failure.'
} finally {
    $resolvedRoot = [IO.Path]::GetFullPath($testRoot)
    if (-not $resolvedRoot.StartsWith($tempBase + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Unsafe test cleanup path'
    }
    if (Test-Path -LiteralPath $resolvedRoot) { Remove-Item -LiteralPath $resolvedRoot -Recurse -Force }
}
