param()

$ErrorActionPreference = "Stop"
$experimentDirectory = $PSScriptRoot
$repository = (Resolve-Path (Join-Path $experimentDirectory "..\..")).Path
$executable = Join-Path $repository "target\release\blocky-evolution.exe"
$stdoutLog = Join-Path $experimentDirectory "comparison-stdout.log"
$stderrLog = Join-Path $experimentDirectory "comparison-stderr.log"
$statusPath = Join-Path $experimentDirectory "comparison-status.json"

foreach ($artifact in @($stdoutLog, $stderrLog, $statusPath)) {
    if (Test-Path -LiteralPath $artifact) {
        throw "Comparison artifact already exists: $artifact"
    }
}
if (-not (Test-Path -LiteralPath $executable)) {
    throw "Release executable not found: $executable"
}

$runs = @("control-seed-2000", "league-40pct-seed-2000")
$generations = @(1, 25, 50)
$startedAt = Get-Date
$completedCommands = 0
$exitCode = 0

Set-Location $repository
try {
    foreach ($runName in $runs) {
        $runDirectory = Join-Path $experimentDirectory "runs\$runName"
        $checkpoint = Join-Path $runDirectory "checkpoint.json"
        foreach ($generation in $generations) {
            $generationId = $generation.ToString("000")
            $validationReport = Join-Path $runDirectory `
                "development-validation-generation-$generationId-depth4.json"
            $validationArguments = @(
                "validate", "--checkpoint", $checkpoint,
                "--report", $validationReport,
                "--generation", "$generation",
                "--workers", "16",
                "--validation-depths", "4",
                "--validation-openings", "20",
                "--validation-seed", "2026072501",
                "--validation-max-game-plies", "200"
            )
            & $executable @validationArguments 1>> $stdoutLog 2>> $stderrLog
            if ($LASTEXITCODE -ne 0) {
                throw "Validation failed for $runName G$generation"
            }
            $completedCommands++

            $benchmarkReport = Join-Path $runDirectory `
                "control-benchmark-generation-$generationId-depth4.json"
            $benchmarkArguments = @(
                "benchmark", "--checkpoint", $checkpoint,
                "--report", $benchmarkReport,
                "--generation", "$generation",
                "--workers", "16",
                "--benchmark-depth", "4",
                "--benchmark-openings", "20",
                "--benchmark-max-game-plies", "200",
                "--random-genomes", "8",
                "--benchmark-seed", "2026072502",
                "--opponent-seed", "2026072503"
            )
            & $executable @benchmarkArguments 1>> $stdoutLog 2>> $stderrLog
            if ($LASTEXITCODE -ne 0) {
                throw "Benchmark failed for $runName G$generation"
            }
            $completedCommands++
        }
    }
}
catch {
    $exitCode = 1
    $_ | Out-String | Add-Content -LiteralPath $stderrLog
}

$endedAt = Get-Date
$stderrBytes = if (Test-Path -LiteralPath $stderrLog) {
    (Get-Item -LiteralPath $stderrLog).Length
} else { 0 }
$state = if ($exitCode -eq 0 -and $completedCommands -eq 12 -and
    $stderrBytes -eq 0) { "completed" } else { "failed" }
$status = [ordered]@{
    state = $state
    exit_code = $exitCode
    completed_commands = $completedCommands
    expected_commands = 12
    started_at_utc = $startedAt.ToUniversalTime().ToString("O")
    ended_at_utc = $endedAt.ToUniversalTime().ToString("O")
    elapsed_seconds = [Math]::Round(($endedAt - $startedAt).TotalSeconds, 3)
    stderr_bytes = $stderrBytes
}
$status | ConvertTo-Json | Set-Content -LiteralPath $statusPath -Encoding utf8
exit $exitCode
