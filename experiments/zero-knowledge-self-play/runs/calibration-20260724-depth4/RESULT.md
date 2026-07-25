# Calibration Result

## Status

Completed successfully on 2026-07-24.

The application completed one full production-configuration training
generation, wrote a resumable checkpoint, ran the intentionally minimal
validation smoke check, and exported the experiment report.

PowerShell classified progress written to standard error as a
`NativeCommandError` in the controlling shell. This did not represent an
application failure: the log reached `Experiment complete`, the checkpoint
contains `next_generation = 1`, and all expected artifacts were written.

## Training workload

| Measurement | Observed value |
| --- | ---: |
| Generations | 1 |
| Population | 32 |
| Swiss rounds | 5 |
| Search depth | 4 |
| Configured workers | 32 |
| Opening pairs per round | 16 |
| Training games | 160 |
| Elapsed training time | 50.312 s |
| Throughput | 3.180 games/s |

The useful concurrency of the current round executor is at most 16 for this
configuration because a round has 16 pairings and each worker plays both
color-swapped games of its assigned pairing sequentially. Configuring 32
workers does not create more than 16 concurrent pairing tasks.

## Game telemetry

| Outcome | Count | Rate |
| --- | ---: | ---: |
| White wins | 41 | 25.625% |
| Black wins | 39 | 24.375% |
| Draws | 80 | 50.000% |
| Total | 160 | 100.000% |

All 80 draws were caused by threefold repetition:

| Draw reason | Count |
| --- | ---: |
| Stalemate | 0 |
| Insufficient material | 0 |
| Threefold repetition | 80 |
| Fifty-move rule | 0 |
| Maximum plies | 0 |

Ply measurements:

| Measurement | Plies |
| --- | ---: |
| Mean | 67.3 |
| Minimum | 20 |
| Median | 64 |
| 95th percentile | 111 |
| Maximum | 136 |

The 200-ply limit did not truncate any calibration game.

## Extrapolated training cost

These estimates assume later generations have the same average cost as the
calibration generation. They are planning estimates, not runtime guarantees.

| Workload | Games | Estimated time |
| --- | ---: | ---: |
| One 100-generation run | 16,000 | 1 h 23 min 51 s |
| Ten 100-generation runs | 160,000 | 13 h 58 min 32 s |

A depth-4 validation with 200 opening pairs contains 400 games. At the observed
depth-4 training throughput, it would take approximately 2 min 6 s per
champion, or 21 minutes for ten champions.

No depth-6 runtime estimate is made from this calibration because alpha-beta
cost does not scale linearly with depth. Depth 6 requires its own benchmark
before the final validation budget can be approved.

## Integrity checks

- Report format: `blocky-evolution`, version 1.
- Report generations: 1.
- Checkpoint `next_generation`: 1.
- Champion ID: 1.
- Champion fitness: 14 half-points.
- Minimal validation games: 2.

Artifact SHA-256 hashes:

```text
checkpoint.json 91FBF962A35D494CFEE788F6263E7F9F156701A189535D4239A443E570CE6E71
report.json     146563948C922D446E117AABD28E91AFD0074FD76206CB2240D69BC2321C84F6
stdout.log      2E4D75A432825821528CD48EDE19BCEC734BF1DF486B1666695847ED7B42F5B9
stderr.log      97C223FF4011E1F7409291ABEAC70700FE7659D96C104E3890B06CA291484DBF
```

## Calibration conclusion

The proposed 10-run training phase is computationally feasible on this
machine: its central estimate is approximately 14 hours of training. The
200-ply limit is not constraining the observed games. Before freezing the
production budget, benchmark depth-6 validation and decide whether to retain
32 configured workers or use the effective maximum of 16.
