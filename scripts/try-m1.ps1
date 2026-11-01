param([switch]$PrepareOnly)
$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$binary = Join-Path $repoRoot 'target/verify-windows/x86_64-pc-windows-msvc/release/dark.exe'
if (!(Test-Path -LiteralPath $binary)) { throw 'Run scripts/verify-windows.ps1 first to build the checked release binary.' }
$reviewDirectory = Join-Path $repoRoot ('.dark/verification/m1/manual-' + [guid]::NewGuid().ToString('N'))
$patch = $reviewDirectory + '.patch'
# A separate clone keeps review actions and snapshots away from the source repository.
& git clone --quiet --no-hardlinks --no-checkout -- $repoRoot $reviewDirectory
if ($LASTEXITCODE -ne 0) { throw 'Could not create the isolated review copy.' }
& git -C $reviewDirectory checkout --quiet --detach cdf607a
if ($LASTEXITCODE -ne 0) { throw 'Could not open the pre-M1 baseline.' }
& git -C $repoRoot diff --binary --no-ext-diff --no-textconv ('--output=' + $patch) cdf607a HEAD --
if ($LASTEXITCODE -ne 0) { throw 'Could not create the M1 review patch.' }
& git -C $reviewDirectory apply --binary -- $patch
if ($LASTEXITCODE -ne 0) { throw 'Could not prepare the M1 changes for review.' }
& git -C $reviewDirectory remote remove origin
if ($LASTEXITCODE -ne 0) { throw 'Could not detach the review copy from its source remote.' }
if ($PrepareOnly) { Write-Output $reviewDirectory; exit 0 }
Write-Host "Review copy: $reviewDirectory"
Write-Host 'a accept, x reject, u restore to opening this session, q quit. The source repository is unchanged.'
Push-Location -LiteralPath $reviewDirectory
try { & $binary --debug; $result = $LASTEXITCODE } finally { Pop-Location }
exit $result
