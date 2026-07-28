param(
    [Parameter(Mandatory = $true)]
    [string]$RunDirectory,

    [Parameter(Mandatory = $true)]
    [string]$ExpectedSourceRevision,

    [string]$LauncherName = "run-training.ps1"
)

$ErrorActionPreference = "Stop"
$experimentDirectory = $PSScriptRoot
$repository = (Resolve-Path (Join-Path $experimentDirectory "..\..")).Path
$resolvedRunDirectory = (Resolve-Path -LiteralPath $RunDirectory).Path
$expectedRunsRoot = (Resolve-Path (Join-Path $experimentDirectory "runs")).Path

if (-not $resolvedRunDirectory.StartsWith(
    $expectedRunsRoot + [IO.Path]::DirectorySeparatorChar,
    [StringComparison]::OrdinalIgnoreCase
)) {
    throw "Run directory must be below $expectedRunsRoot"
}

$launcher = Join-Path $resolvedRunDirectory $LauncherName
$provenancePath = Join-Path $resolvedRunDirectory "binary-provenance.json"
if (-not (Test-Path -LiteralPath $launcher)) {
    throw "Run launcher not found: $launcher"
}
if (Test-Path -LiteralPath $provenancePath) {
    throw "Binary provenance already exists: $provenancePath"
}

Set-Location $repository
& git cat-file -e "$ExpectedSourceRevision^{commit}"
if ($LASTEXITCODE -ne 0) {
    throw "Expected source revision is not a Git commit"
}
$resolvedSourceRevision = (& git rev-parse "$ExpectedSourceRevision^{commit}").Trim()
$currentHead = (& git rev-parse HEAD).Trim()
$sourcePaths = @("Cargo.toml", "Cargo.lock", "build.rs", "src", "crates")

& git diff --quiet $resolvedSourceRevision -- @sourcePaths
if ($LASTEXITCODE -ne 0) {
    throw "Workspace source differs from expected revision $resolvedSourceRevision"
}
$untrackedSource = & git status --porcelain --untracked-files=all -- @sourcePaths
if ($untrackedSource) {
    throw "Workspace contains untracked or modified source paths: $untrackedSource"
}

& cargo build --release -p blocky-evolution
if ($LASTEXITCODE -ne 0) {
    throw "Release build failed"
}

$executable = Join-Path $repository "target\release\blocky-evolution.exe"
if (-not (Test-Path -LiteralPath $executable)) {
    throw "Release executable not found after build: $executable"
}
$binary = Get-Item -LiteralPath $executable
$provenance = [ordered]@{
    format = "blocky-evolution-binary-provenance"
    version = 1
    recorded_at_utc = (Get-Date).ToUniversalTime().ToString("O")
    expected_source_revision = $resolvedSourceRevision
    repository_head = $currentHead
    verified_source_paths = $sourcePaths
    build_profile = "release"
    cargo_version = (& cargo --version).Trim()
    rustc_version = (& rustc --version).Trim()
    executable = "target/release/blocky-evolution.exe"
    executable_length_bytes = $binary.Length
    executable_last_write_time_utc = $binary.LastWriteTimeUtc.ToString("O")
    executable_sha256 = (Get-FileHash -LiteralPath $executable `
        -Algorithm SHA256).Hash
}
$temporaryProvenance = "$provenancePath.tmp"
$provenance | ConvertTo-Json -Depth 4 |
    Set-Content -LiteralPath $temporaryProvenance -Encoding utf8
Move-Item -LiteralPath $temporaryProvenance -Destination $provenancePath

& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $launcher
exit $LASTEXITCODE
