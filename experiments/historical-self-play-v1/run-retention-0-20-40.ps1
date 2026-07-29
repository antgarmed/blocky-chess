param()
$ErrorActionPreference = "Stop"
$experimentDirectory = $PSScriptRoot
$repository = (Resolve-Path (Join-Path $experimentDirectory "..\..")).Path
$executable = Join-Path $repository "target\release\blocky-evolution.exe"
$manifest = Join-Path $experimentDirectory "retention-panel-0-20-40.json"
$report = Join-Path $experimentDirectory "retention-panel-0-20-40-report.json"
$stdoutLog = Join-Path $experimentDirectory "retention-0-20-40-stdout.log"
$stderrLog = Join-Path $experimentDirectory "retention-0-20-40-stderr.log"
$statusPath = Join-Path $experimentDirectory "retention-0-20-40-status.json"
foreach ($artifact in @($report, $stdoutLog, $stderrLog, $statusPath)) { if (Test-Path -LiteralPath $artifact) { throw "Artifact already exists: $artifact" } }
if (-not (Test-Path -LiteralPath $executable)) { throw "Release executable not found: $executable" }
$startedAt = Get-Date
Set-Location $repository
& $executable retention-benchmark --manifest $manifest --report $report 1> $stdoutLog 2> $stderrLog
$exitCode = $LASTEXITCODE
$endedAt = Get-Date
$stderrBytes = (Get-Item $stderrLog).Length
$status = [ordered]@{ state = if ($exitCode -eq 0 -and (Test-Path $report) -and $stderrBytes -eq 0) { "completed" } else { "failed" }; exit_code = $exitCode; started_at_utc = $startedAt.ToUniversalTime().ToString("O"); ended_at_utc = $endedAt.ToUniversalTime().ToString("O"); elapsed_seconds = [Math]::Round(($endedAt - $startedAt).TotalSeconds, 3); stderr_bytes = $stderrBytes }
$status | ConvertTo-Json | Set-Content -LiteralPath $statusPath -Encoding utf8
exit $exitCode
