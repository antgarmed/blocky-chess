# Depth-6 Validation Calibration Result

## Scope

This calibration resumed the completed one-generation checkpoint from
`calibration-20260724-depth4`. It did not repeat self-play training.

The validation used:

- search depth: 6;
- opening pairs: 10;
- games: 20 (colors reversed for each opening);
- workers: 16;
- maximum game length: 200 plies;
- held-out validation seed: 424244.

## Observed result

- elapsed validation time: 963.809 seconds (16 minutes 3.809 seconds);
- throughput: 0.021 games/second;
- candidate score: 4 half-points (2 points);
- reference score: 36 half-points (18 points);
- decision: candidate rejected;
- white wins / black wins / draws: 8 / 8 / 4;
- all four draws were caused by threefold repetition;
- mean plies: 68.0;
- minimum / median / p95 / maximum plies: 25 / 64 / 96 / 112;
- no game reached the 200-ply limit.

The candidate result is exploratory and is not a sealed confirmatory result.
This run is used to size the final validation workload.

## Runtime projection

The calibration ran all 10 opening-pair tasks concurrently. With 16 workers,
the estimated number of sequential task batches is:

| Opening pairs | Batches | Estimated time per champion |
| ---: | ---: | ---: |
| 50 | 4 | about 64 minutes |
| 100 | 7 | about 112 minutes |
| 200 | 13 | about 209 minutes |

These are rough wall-clock projections based on one observed batch. Actual
time is controlled by the slowest opening in each batch and may vary with
thermal throttling and concurrent system load.

## Decision

Use **50 opening pairs (100 games) at depth 6** for each final champion.

This follows the predeclared calibration rule to use 50 opening pairs or
reconsider the workload when the 10-pair calibration takes more than 15
minutes. At the observed rate, depth-6 validation is expected to take about
64 minutes per champion, or about 10 hours 43 minutes for 10 champions.

## Artifact integrity

- `report.json` SHA-256:
  `C47D70637D0B95C8C5A30B9F32A492F7B0E480C3ED4EC79C4737B939BBB93745`
- `stdout.log` SHA-256:
  `3C2779AFD6DEC4B7A0F6535DF1B05DB1ECF17A9CC66B0AFAB491A81314971525`
- `stderr.log` SHA-256:
  `DF990AF7A9F28F069AA2290AC1D428B7B520A502BA00AD8A94DBC57137DFC57B`
