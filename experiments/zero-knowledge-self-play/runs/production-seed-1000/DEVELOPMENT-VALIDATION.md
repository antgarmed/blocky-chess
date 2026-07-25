# Seed 1000 Development Benchmarks

## Purpose

These exploratory benchmarks measure the best individual from selected
generations against `EvaluationConfig::default()` and two random controls.
They do not use the sealed final-validation seed and must not influence
champion selection or frozen hyperparameters.

## Default-reference configuration

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

## Default-reference results

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

## Random-control configuration

The same candidates were evaluated with the standalone `benchmark` command:

- Search depth: `4`
- Opening pairs per control and candidate: `20`
- Random-legal games per candidate: `40`
- Random-genome opponents: `8`
- Random-genome games per candidate: `320`
- Benchmark opening seed: `2026072502`
- Random-genome seed: `2026072503`
- Workers: `16`
- Maximum game length: `200` plies
- Colors: swapped within every opening pair

`RandomLegal` samples uniformly from legal moves and acts only as a competence
floor. Each `RandomGenome` uses the same alpha-beta depth as the candidate.
The eight normalized genomes and all game results are recorded in every JSON
report. All candidates use the same control genomes and the same openings.

Each snapshot was run with:

```text
blocky-evolution benchmark
  --checkpoint checkpoint.json
  --report control-benchmark-generation-NNN-depth4.json
  --generation N
  --workers 16
  --benchmark-depth 4
  --benchmark-openings 20
  --benchmark-max-game-plies 200
  --random-genomes 8
  --benchmark-seed 2026072502
  --opponent-seed 2026072503
```

## Random-control results

| Candidate | RandomLegal | W-D-L | RandomGenome ensemble | W-D-L | Ensemble score |
| --- | ---: | ---: | ---: | ---: | ---: |
| Generation 1 | 79-1 | 39-1-0 | 394-246 | 119-156-45 | 61.56% |
| Generation 25 | 78-2 | 38-2-0 | 462-178 | 163-136-21 | 72.19% |
| Generation 50 | 80-0 | 40-0-0 | 467-173 | 169-129-22 | 72.97% |
| Generation 75 | 77-3 | 37-3-0 | 459-181 | 165-129-26 | 71.72% |
| Generation 100 | 77-3 | 37-3-0 | 477-163 | 173-131-16 | 74.53% |
| Best-ever | 79-1 | 39-1-0 | 455-185 | 157-141-22 | 71.09% |

RandomLegal is effectively saturated from generation 1 and cannot measure
subsequent progress. The RandomGenome ensemble remains discriminative.

## Combined interpretation

The strongest observed snapshot is generation 25, but it scores only `27.50%`
against `Default`. Performance then declines through generations 50, 75, and
100. The training `best-ever` candidate and the generation-100 candidate are
different individuals, yet both score `16.25%` on this benchmark.

This run does not show convergence toward the external reference. It shows a
small early improvement followed by deterioration on the fixed development
set. Swiss fitness is not comparable across generations and does not track
external playing strength here: candidates with a Swiss fitness of `16` score
both `20.00%` and `16.25%` against `Default`.

The random controls change the diagnosis. Generation 1 already scores
`61.56%` against the fixed RandomGenome ensemble. Generations 25 through 100
score between `71.72%` and `74.53%`, with generation 100 producing the best
ensemble result. The evolutionary run therefore shows real improvement
relative to the knowledge-free random distribution.

Progress relative to RandomGenome and progress relative to `Default` diverge:

| Generation | Default score | RandomGenome score |
| --- | ---: | ---: |
| 1 | 22.50% | 61.56% |
| 25 | 27.50% | 72.19% |
| 50 | 20.00% | 72.97% |
| 75 | 18.75% | 71.72% |
| 100 | 16.25% | 74.53% |

This is evidence of objective misalignment or specialization, rather than an
absence of learning. Contemporary self-play selects configurations that become
stronger than random evaluation configurations while drifting farther from
the hand-designed default reference. A future training revision should test
an external anchor or hall-of-fame mechanism without discarding the useful
self-play signal.

Both benchmarks are intentionally small and use one fixed development set per
reference class, so exact percentages remain exploratory. The deficits against
`Default` and gains against RandomGenome are nevertheless consistent across
the sampled generations. The sealed final-validation seed has not been used.

## Artifact hashes

```text
D3B15AE8B3F94F6A5B7A97A14CDA7E1EEDFBD9029C8F0397DD354051DC320188  development-validation-generation-001-depth4.json
F7586B6765C274F43371052E4C2425F0B2BF32470A8DE1AC5D03B233A70FE3AD  development-validation-generation-025-depth4.json
92135D5DA1B45FF0BFF47FD78F5F1816AFAF979A24F8358807F0F75F780537B4  development-validation-generation-050-depth4.json
7FCCA6D029005EAF2242DE4F6DE254C9D75E8E5C9C701BFE45C6E3155118CEBD  development-validation-generation-075-depth4.json
F28A62995688AF4381E2713488AB200B002AF6F4EAD7B442B7E7182589405AF5  development-validation-generation-100-depth4.json
FD2076F7DD0ECAEE1345F7FD0A0AE70ABFB0C2598306A0326F72625EC708C590  development-validation-depth4.json
9D10C2304019CB457F73463F2D93474023D4F2719E0E8CAEBF20A02738A5093E  control-benchmark-generation-001-depth4.json
CD2E85F29CE4C9C1A6C6717316B7275AA08CD559466AA3FB91CF4959B7293EB7  control-benchmark-generation-025-depth4.json
342FF012D3E205AE408A8683F61C3CEE7D7DA939E14E6F50BA2AB03559118C4D  control-benchmark-generation-050-depth4.json
A0ADF7B503152180D865A75D4B88548224A66744D9CB96192C59432FF9E5C226  control-benchmark-generation-075-depth4.json
297828E22C8B4291818DABD899C83C7FC62D3F8D16C9D9C50D22861AE4A12AA3  control-benchmark-generation-100-depth4.json
1AA45D27BA8E05B52E29DC4CEC61447A6450D30C92BAD2B67FE44AC51E7C5CA4  control-benchmark-best-ever-depth4.json
```
