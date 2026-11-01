param([string]$OutputDirectory = '.dark/verification/m1/word-diff')
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $root
$previousTargetDirectory = $env:CARGO_TARGET_DIR
try {
    $env:CARGO_TARGET_DIR = Join-Path $root 'target/word-diff-probe'
    # Build dependencies using the unchanged project release profile.
    cargo build --release -p similar -p unicode-segmentation -p windows-sys
    if ($LASTEXITCODE -ne 0) { throw 'Release dependency build failed' }
    $metadata = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    $deps = Join-Path $metadata.target_directory 'release/deps'
    $externs = @()
    foreach ($crate in @('similar', 'unicode_segmentation', 'windows_sys')) {
        $matches = @(Get-ChildItem -LiteralPath $deps -Filter "lib$crate-*.rlib")
        if ($matches.Count -ne 1) { throw "Expected one release rlib for $crate in $deps; use a fresh CARGO_TARGET_DIR" }
        $externs += @('--extern', "$crate=$($matches[0].FullName)")
    }
    New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
    $output = (Resolve-Path -LiteralPath $OutputDirectory).Path
    $results = @()
    foreach ($variant in @('baseline', 'with-similar')) {
        $binary = Join-Path $output "$variant.exe"
        $arguments = @('--edition=2021', 'tests/bench/word-diff-probe.rs', '-C', 'opt-level=z', '-C', 'lto=fat', '-C', 'codegen-units=1', '-C', 'panic=abort', '-C', 'strip=symbols', '-L', "dependency=$deps", '-o', $binary) + $externs
        if ($variant -eq 'with-similar') { $arguments += @('--cfg', 'word_diff') }
        rustc @arguments
        if ($LASTEXITCODE -ne 0) { throw "Probe build failed: $variant" }
        for ($run = 1; $run -le 5; $run++) {
            $measurement = & $binary
            if ($LASTEXITCODE -ne 0) { throw "Probe failed: $variant run $run" }
            $line = "variant=$variant run=$run binary_bytes=$((Get-Item -LiteralPath $binary).Length) $measurement"
            $results += $line
            $line
        }
    }
    $results | Set-Content -Encoding utf8 -LiteralPath (Join-Path $output 'results.txt')
} finally {
    $env:CARGO_TARGET_DIR = $previousTargetDirectory
    Pop-Location
}

