param(
    [int]$BatchNumber = 1,
    [int]$DurationSeconds = 3600
)

$ErrorActionPreference = "Stop"
$runDirectory = $PSScriptRoot
$repository = (Resolve-Path (Join-Path $runDirectory "..\..\..")).Path
$executable = Join-Path $repository "target\release\blocky-evolution.exe"
$checkpoint = Join-Path $runDirectory "checkpoint.json"
$report = Join-Path $runDirectory "report-depth4.json"
$batchId = $BatchNumber.ToString("000")
$stdoutLog = Join-Path $runDirectory "batch-$batchId-stdout.log"
$stderrLog = Join-Path $runDirectory "batch-$batchId-stderr.log"
$statusPath = Join-Path $runDirectory "batch-$batchId-status.json"

if (-not (Test-Path -LiteralPath $executable)) {
    throw "Release executable not found: $executable"
}
if ((Test-Path -LiteralPath $stdoutLog) -or
    (Test-Path -LiteralPath $stderrLog) -or
    (Test-Path -LiteralPath $statusPath)) {
    throw "Batch $batchId artifacts already exist"
}

$trainingArguments = @(
    "train",
    "--generations", "100",
    "--population-size", "32",
    "--swiss-rounds", "5",
    "--elite-count", "2",
    "--parent-candidate-count", "3",
    "--gene-mutation-probability", "0.15",
    "--strong-mutation-probability", "0.02",
    "--mutation-step", "0.10",
    "--strong-mutation-step", "0.50",
    "--workers", "16",
    "--search-depth", "4",
    "--max-game-plies", "200",
    "--training-seed", "1000",
    "--opening-min-plies", "4",
    "--opening-max-plies", "10",
    "--max-opening-attempts", "100",
    "--validation-depths", "4",
    "--validation-openings", "200",
    "--validation-max-game-plies", "200",
    "--validation-seed", "6215332838309450821",
    "--validation-opening-min-plies", "4",
    "--validation-opening-max-plies", "10",
    "--validation-max-opening-attempts", "100",
    "--validation-minimum-margin-half-points", "1",
    "--checkpoint", $checkpoint,
    "--checkpoint-every", "1",
    "--report", $report
)
if (Test-Path -LiteralPath $checkpoint) {
    $trainingArguments += @("--resume", $checkpoint)
}

function Quote-CommandArgument {
    param([string]$Value)
    if ($Value -notmatch '[\s"]') {
        return $Value
    }
    return '"' + ($Value -replace '(\\*)"', '$1$1\"' -replace '(\\+)$', '$1$1') + '"'
}

function Read-CheckpointGeneration {
    if (-not (Test-Path -LiteralPath $checkpoint)) {
        return 0
    }
    try {
        $document = Get-Content -LiteralPath $checkpoint -Raw | ConvertFrom-Json
        return [int]$document.state.next_generation
    }
    catch {
        return $null
    }
}

function Write-Status {
    param(
        [string]$State,
        [string]$Reason,
        [int]$Generation,
        [datetime]$StartedAt,
        [datetime]$EndedAt,
        [int]$ProcessId
    )
    $status = [ordered]@{
        batch = $BatchNumber
        state = $State
        reason = $Reason
        process_id = $ProcessId
        started_at_utc = $StartedAt.ToUniversalTime().ToString("O")
        ended_at_utc = $EndedAt.ToUniversalTime().ToString("O")
        elapsed_seconds = [Math]::Round(($EndedAt - $StartedAt).TotalSeconds, 3)
        checkpoint_next_generation = $Generation
        stdout_log = [IO.Path]::GetFileName($stdoutLog)
        stderr_log = [IO.Path]::GetFileName($stderrLog)
    }
    $temporaryStatus = "$statusPath.tmp"
    $status | ConvertTo-Json | Set-Content -LiteralPath $temporaryStatus -Encoding utf8
    Move-Item -LiteralPath $temporaryStatus -Destination $statusPath
}

Set-Location $repository
$argumentText = ($trainingArguments | ForEach-Object { Quote-CommandArgument $_ }) -join " "
$commandText = '"' + $executable + '" ' + $argumentText +
    ' 1>"' + $stdoutLog + '" 2>"' + $stderrLog + '"'
$startInfo = [System.Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = $env:ComSpec
$startInfo.Arguments = "/d /s /c `"$commandText`""
$startInfo.UseShellExecute = $false
$startInfo.CreateNoWindow = $true

$startedAt = Get-Date
$deadline = $startedAt.AddSeconds($DurationSeconds)
$initialGeneration = Read-CheckpointGeneration
if ($null -eq $initialGeneration) {
    throw "Existing checkpoint is not valid JSON"
}
$process = [System.Diagnostics.Process]::Start($startInfo)
$reason = "process_exited"

try {
    while (-not $process.HasExited) {
        Start-Sleep -Seconds 5
        $generation = Read-CheckpointGeneration
        if ($null -eq $generation) {
            continue
        }
        if ($generation -ge 100) {
            $reason = "training_complete"
            & taskkill.exe /PID $process.Id /T /F | Out-Null
            break
        }
        if ((Get-Date) -ge $deadline -and $generation -gt $initialGeneration) {
            $reason = "time_box_complete"
            & taskkill.exe /PID $process.Id /T /F | Out-Null
            break
        }
    }
    $process.WaitForExit()
}
finally {
    if (-not $process.HasExited) {
        & taskkill.exe /PID $process.Id /T /F | Out-Null
        $process.WaitForExit()
    }
}

$endedAt = Get-Date
$finalGeneration = Read-CheckpointGeneration
if ($null -eq $finalGeneration) {
    $finalGeneration = 0
    $reason = "invalid_checkpoint"
}
$state = if ($reason -in @("time_box_complete", "training_complete")) {
    "completed"
}
else {
    "failed"
}
Write-Status $state $reason $finalGeneration $startedAt $endedAt $process.Id
