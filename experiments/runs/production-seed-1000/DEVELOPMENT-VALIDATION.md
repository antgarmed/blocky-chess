# Seed 1000 Development Validation

## Purpose

This exploratory benchmark measures the best individual from selected
generations against `EvaluationConfig::default()`. It does not use the sealed
final-validation seed and must not influence champion selection or frozen
hyperparameters.

## Configuration

- Training checkpoint: `checkpoint.json`
- Training seed: `1000`
- Human-numbered generations: `1`, `25`, `50`, `75`, `100`
- Reference: `EvaluationConfig::default()`
- Search depth: `4`
- Opening pairs per candidate: `20`
- Games per candidate: `40`
- Development seed: `2026072501`
- Workers: `16`
- Maximum game length: `200` plies
- Colors: swapped within every opening pair

Each snapshot was run with:

```text
blocky-evolution validate
  --checkpoint checkpoint.json
  --report development-validation-generation-NNN-depth4.json
  --generation N
  --workers 16
  --validation-depths 4
  --validation-openings 20
  --validation-seed 2026072501
  --validation-max-game-plies 200
```

The separately selected `best-ever` candidate was evaluated with the same
configuration and without `--generation`.

## Results

Scores use half-points, where a win is `2`, a draw is `1`, and a loss is `0`.
Every row therefore distributes `80` half-points across both players.

| Candidate | Stored index | ID | Swiss fitness | Candidate | Default | W-D-L | Score |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Generation 1 | 0 | 10 | 14 | 18 | 62 | 3-12-25 | 22.50% |
| Generation 25 | 24 | 729 | 14 | 22 | 58 | 4-14-22 | 27.50% |
| Generation 50 | 49 | 1477 | 16 | 16 | 64 | 2-12-26 | 20.00% |
| Generation 75 | 74 | 2224 | 13 | 15 | 65 | 2-11-27 | 18.75% |
| Generation 100 | 99 | 2944 | 14 | 13 | 67 | 2-9-29 | 16.25% |
| Best-ever | n/a | 845 | 16 | 13 | 67 | 1-11-28 | 16.25% |

All candidates were rejected by the configured per-depth majority rule.

## Interpretation

The strongest observed snapshot is generation 25, but it scores only `27.50%`
against `Default`. Performance then declines through generations 50, 75, and
100. The training `best-ever` candidate and the generation-100 candidate are
different individuals, yet both score `16.25%` on this benchmark.

This run does not show convergence toward the external reference. It shows a
small early improvement followed by deterioration on the fixed development
set. Swiss fitness is not comparable across generations and does not track
external playing strength here: candidates with a Swiss fitness of `16` score
both `20.00%` and `16.25%` against `Default`.

The benchmark is intentionally small and uses one development opening set, so
the exact percentages remain exploratory. The large deficit is nevertheless
consistent across every sampled generation. The sealed final-validation seed
has not been used.

## Artifact hashes

```text
D3B15AE8B3F94F6A5B7A97A14CDA7E1EEDFBD9029C8F0397DD354051DC320188  development-validation-generation-001-depth4.json
F7586B6765C274F43371052E4C2425F0B2BF32470A8DE1AC5D03B233A70FE3AD  development-validation-generation-025-depth4.json
92135D5DA1B45FF0BFF47FD78F5F1816AFAF979A24F8358807F0F75F780537B4  development-validation-generation-050-depth4.json
7FCCA6D029005EAF2242DE4F6DE254C9D75E8E5C9C701BFE45C6E3155118CEBD  development-validation-generation-075-depth4.json
F28A62995688AF4381E2713488AB200B002AF6F4EAD7B442B7E7182589405AF5  development-validation-generation-100-depth4.json
FD2076F7DD0ECAEE1345F7FD0A0AE70ABFB0C2598306A0326F72625EC708C590  development-validation-depth4.json
```
