# Small-Anchor Depth-4 Calibration

The calibration completed successfully using the intended production
population, self-play schedule, search depth, worker count, and small Default
anchor. It is an operational measurement and is not evidence for or against
the experiment hypotheses.

## Configuration

| Parameter | Value |
| --- | ---: |
| Generations | 1 |
| Population | 32 |
| Swiss rounds | 5 |
| Self-play games | 160 |
| Default-anchor weight | 10% |
| Default-anchor opening pairs per individual | 1 |
| Default-anchor games | 64 |
| Total games | 224 |
| Search depth | 4 |
| Workers | 16 |

## Result

| Measurement | Result |
| --- | ---: |
| Wall-clock duration | 82.874 s |
| Generation-reported duration | 82.593 s |
| Throughput | 2.712 games/s |
| Aggregate anchor score | 13 / 128 half-points |
| Best individual ID | 24 |
| Best self-play component | 13 / 20 half-points |
| Best anchor component | 2 / 4 half-points |
| Best composite selection score | 1270 / 2000 units |
| Checkpoint format | `blocky-evolution` v2 |
| Checkpoint next generation | 1 |
| Standard error bytes | 0 |

The checkpoint persisted all 32 evaluated individuals with separate
`self_play_score`, `default_anchor_score`, and `selection_score` fields.
Validation was intentionally skipped.

Compared with the original 50.312-second zero-knowledge calibration, the
small-anchor generation took approximately 64.7% longer while playing 40%
more games. A linear projection estimates about 2 hours 18 minutes for 100
generations, although production throughput may vary with the evolved
positions and concurrent desktop workload.

## Integrity

```text
B96931EFD8169D192B0D180230318025B0221F3B2EBBB71D34565D6D41B7D96B  checkpoint.json
```
