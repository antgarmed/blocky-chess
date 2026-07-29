param()

$ErrorActionPreference = "Stop"
$runDirectory = $PSScriptRoot
$repository = (Resolve-Path (Join-Path $runDirectory "..\..\..\..")).Path
$executable = Join-Path $repository "target\release\blocky-evolution.exe"
$checkpoint = Join-Path $runDirectory "checkpoint.json"
$stdoutLog = Join-Path $runDirectory "stdout.log"
$stderrLog = Join-Path $runDirectory "stderr.log"
$statusPath = Join-Path $runDirectory "status.json"

if (-not (Test-Path -LiteralPath $executable)) { throw "Release executable not found: $executable" }
foreach ($artifact in @($checkpoint, $stdoutLog, $stderrLog, $statusPath)) {
    if (Test-Path -LiteralPath $artifact) { throw "Run artifact already exists: $artifact" }
}

$arguments = @(
    "train", "--training-only", "--generations", "50",
    "--population-size", "32", "--swiss-rounds", "5",
    "--elite-count", "2", "--parent-candidate-count", "3",
    "--gene-mutation-probability", "0.15",
    "--strong-mutation-probability", "0.02",
    "--mutation-step", "0.10", "--strong-mutation-step", "0.50",
    "--search-depth", "4", "--max-game-plies", "200",
    "--training-seed", "2000", "--opening-min-plies", "4",
    "--opening-max-plies", "10",
    "--historical-weight-percent", "20",
    "--historical-opponents", "4", "--historical-opening-pairs", "1",
    "--historical-insertion-cadence", "5", "--historical-max-size", "16",
    "--workers", "16", "--checkpoint", $checkpoint,
    "--checkpoint-every", "1"
)

function Quote-Argument([string]$value) {
    if ($value -notmatch '[\s"]') { return $value }
    return '"' + ($value -replace '(\\*)"', '$1$1\"' -replace '(\\+)$', '$1$1') + '"'
}

Set-Location $repository
$argumentText = ($arguments | ForEach-Object { Quote-Argument $_ }) -join " "
$commandText = '"' + $executable + '" ' + $argumentText +
    ' 1>"' + $stdoutLog + '" 2>"' + $stderrLog + '"'
$startInfo = [System.Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = $env:ComSpec
$startInfo.Arguments = "/d /s /c `"$commandText`""
$startInfo.UseShellExecute = $false
$startInfo.CreateNoWindow = $true

$startedAt = Get-Date
$process = [System.Diagnostics.Process]::Start($startInfo)
$process.WaitForExit()
$endedAt = Get-Date
$nextGeneration = $null
if (Test-Path -LiteralPath $checkpoint) {
    try { $nextGeneration = [int](Get-Content -LiteralPath $checkpoint -Raw | ConvertFrom-Json).state.next_generation } catch {}
}
$stderrBytes = if (Test-Path -LiteralPath $stderrLog) { (Get-Item -LiteralPath $stderrLog).Length } else { -1 }
$state = if ($process.ExitCode -eq 0 -and $nextGeneration -eq 50 -and $stderrBytes -eq 0) { "completed" } else { "failed" }
$status = [ordered]@{
    state = $state; process_id = $process.Id; exit_code = $process.ExitCode
    started_at_utc = $startedAt.ToUniversalTime().ToString("O")
    ended_at_utc = $endedAt.ToUniversalTime().ToString("O")
    elapsed_seconds = [Math]::Round(($endedAt - $startedAt).TotalSeconds, 3)
    checkpoint_next_generation = $nextGeneration; stderr_bytes = $stderrBytes
}
$status | ConvertTo-Json | Set-Content -LiteralPath $statusPath -Encoding utf8
if ($state -ne "completed") { exit 1 }
