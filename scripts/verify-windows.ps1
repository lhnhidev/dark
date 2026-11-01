param([string]$OutputDirectory = '')
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $repoRoot
if (!$OutputDirectory) { $OutputDirectory = Join-Path $repoRoot '.dark/verification/m0-review/after/windows' }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$cargo = Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe'
if (!(Test-Path -LiteralPath $cargo)) {
    $cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
    if (!$cargoCommand) { throw 'Cargo is unavailable. Install Rust before running verification.' }
    $cargo = $cargoCommand.Source
}
$script:Failures = 0
$rustc = Join-Path (Split-Path $cargo -Parent) 'rustc.exe'
if (!(Test-Path -LiteralPath $rustc)) { throw 'rustc.exe is unavailable beside Cargo.' }
$rustInfo = & $rustc -vV
if ($LASTEXITCODE -ne 0) { throw 'Could not determine the Rust host target.' }
$hostTriple = (($rustInfo | Where-Object { $_ -like 'host: *' }) -replace '^host: ', '').Trim()
if (!$hostTriple -or $hostTriple -notmatch 'windows') { throw "Expected a Windows Rust host, got: $hostTriple" }
$targetDirectory = Join-Path $repoRoot 'target/verify-windows'
$binaryDirectory = Join-Path $targetDirectory $hostTriple
$environmentRecord = "CARGO_TARGET_DIR=$targetDirectory`nexplicit_host_target=$hostTriple`n"
[IO.File]::WriteAllText((Join-Path $OutputDirectory 'environment.txt'), $environmentRecord, [Text.UTF8Encoding]::new($false))
Write-Output $environmentRecord
function Quote-NativeArgument([string]$Value) {
    if ($Value -notmatch '[\s"]') { return $Value }
    return '"' + (($Value -replace '(\\*)"', '$1$1\"') -replace '(\\+)$', '$1$1') + '"'
}
function Invoke-Check([string]$Name, [string]$Program, [string[]]$Arguments) {
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = $Program
    $start.Arguments = ($Arguments | ForEach-Object { Quote-NativeArgument $_ }) -join ' '
    $start.WorkingDirectory = $repoRoot
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.EnvironmentVariables['CARGO_TARGET_DIR'] = $targetDirectory
    $process = New-Object Diagnostics.Process
    $process.StartInfo = $start
    try {
        if (!$process.Start()) { throw "Could not start $Program" }
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $code = $process.ExitCode
        $record = '$ ' + $Program + ' ' + $start.Arguments + "`nstdout:`n" + $stdout.Result + "`nstderr:`n" + $stderr.Result + "`nexit_code=$code`n"
        [IO.File]::WriteAllText((Join-Path $OutputDirectory "$Name.txt"), $record, [Text.UTF8Encoding]::new($false))
        Write-Output $record
        if ($code -ne 0) { $script:Failures++ }
    } finally { $process.Dispose() }
}
Invoke-Check 'fmt' $cargo @('fmt', '--check')
Invoke-Check 'clippy' $cargo @('clippy', '--locked', '--all-targets', '--target', $hostTriple, '--', '-D', 'warnings')
Invoke-Check 'test' $cargo @('test', '--locked', '--target', $hostTriple)
Invoke-Check 'test-release' $cargo @('test', '--locked', '--release', '--target', $hostTriple, '--', '--nocapture')
Invoke-Check 'build-debug' $cargo @('build', '--locked', '--target', $hostTriple)
Invoke-Check 'build-release' $cargo @('build', '--locked', '--release', '--target', $hostTriple)
$binary = Join-Path $binaryDirectory 'release/dark.exe'
if (Test-Path -LiteralPath $binary) {
    $bytes = (Get-Item -LiteralPath $binary).Length
    $sizeRecord = "binary_size_bytes=$bytes`n"
    [IO.File]::WriteAllText((Join-Path $OutputDirectory 'binary-size.txt'), $sizeRecord, [Text.UTF8Encoding]::new($false))
    Write-Output $sizeRecord
    if ($bytes -ge 3000000) { $script:Failures++ }
    Invoke-Check 'self-test' $binary @('--self-test')
    Invoke-Check 'help' $binary @('--help')
    Invoke-Check 'version' $binary @('--version')
} else {
    Write-Output "Missing expected release binary: $binary"
    $script:Failures++
}
Invoke-Check 'terminal' (Join-Path $PSHOME 'powershell.exe') @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $repoRoot 'tests/ram/windows-console.ps1'), '-TargetDirectory', $binaryDirectory)
if ($script:Failures) { exit 1 }
exit 0
