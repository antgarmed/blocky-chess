# Small-Anchor Seed 1000 Training Result

Training completed the frozen 100-generation target using contemporary
self-play with a 10% `EvaluationConfig::default()` anchor.

## Configuration

| Parameter | Value |
| --- | ---: |
| Generations | 100 |
| Population | 32 |
| Swiss rounds per generation | 5 |
| Self-play games | 16,000 |
| Default-anchor weight | 10% |
| Default-anchor opening pairs per individual and generation | 1 |
| Default-anchor games | 6,400 |
| Authoritative training games | 22,400 |
| Search depth | 4 |
| Workers | 16 |
| Training seed | 1000 |

## Operational result

| Measurement | Result |
| --- | ---: |
| Recorded/observed elapsed time | 7,270.031 s (2 h 01 min 10.031 s) |
| Mean authoritative generation time | 71.957 s |
| Mean authoritative throughput | 3.211 games/s |
| Aggregate candidate anchor score | 1,790 / 12,800 half-points (13.984%) |
| Completed batches | 4 |
| Infrastructure interruptions | 2 |

Batch 002 ended through an unexplained external process termination at
generation 64. Batch 003 failed while replacing a checkpoint temporarily
locked by Windows at generation 70. Atomic checkpoints preserved every
complete accepted generation; deterministic resumption recomputed discarded
partial work.

## Frozen champion

| Measurement | Result |
| --- | ---: |
| Champion ID | 1446 |
| First appearance | Generation 49 |
| Best-record generation | Generation 49 |
| Self-play component | 16 / 20 half-points |
| Default-anchor component | 2 / 4 half-points |
| Composite selection score | 1,540 / 2,000 units |

The selection score is not an absolute strength measurement and must not be
compared across generations. Development benchmarks against fixed opponents
and openings provide the appropriate comparison with the zero-knowledge
experiment.

## Integrity

```text
0AB961A9A8E6C2F86A006397CE2E4BE3B4696BE3A1ED15D0F849A2C190C36550  checkpoint.json
```
