# Default-Anchored Self-Play Experiment

## 1. Purpose

This experiment tests whether a small external anchor can prevent the
objective drift observed during knowledge-free contemporary self-play without
removing the useful learning signal produced by self-play.

The primary research question is:

> Does adding a low-weight `EvaluationConfig::default()` anchor to
> contemporary self-play preserve or improve performance against the default
> evaluation while retaining improvement over knowledge-free random
> evaluation configurations?

This is a separate experiment from the original
[`zero-knowledge-self-play`](../zero-knowledge-self-play/EXPERIMENT.md)
protocol. Its runs and conclusions must be reported separately.

## 2. Classification and Scope

The initial population remains randomly generated. Evolution still uses the
same genome, search, self-play, Swiss pairing, selection, crossover, and
mutation mechanisms. No opening book, historical game database, endgame
tablebase, external engine, or manually labelled chess position is introduced.

However, this experiment is **not strictly zero-knowledge**:
`EvaluationConfig::default()` contains manually chosen chess knowledge and is
used during training. The appropriate description is **default-anchored
self-play** or **knowledge-anchored self-play**.

`Default` is an anchor, not a population member:

- it cannot reproduce, mutate, receive Swiss pairings, or become champion;
- every evolved individual is still produced by the evolutionary operators;
- contemporary self-play remains the dominant component of the objective;
- the anchor supplies a small absolute signal intended to limit specialization
  and forgetting.

## 3. Motivation

The completed seed-1000 development curve from the zero-knowledge experiment
showed two simultaneous effects:

- score against a fixed `RandomGenome` ensemble increased from `61.56%` at
  generation 1 to `74.53%` at generation 100;
- score against `Default` peaked at `27.50%` at generation 25 and declined to
  `16.25%` at generation 100.

This is evidence that evolution learned something useful relative to the
knowledge-free random distribution, but that the contemporary self-play
objective was misaligned with the external reference. The new experiment
tests a targeted response to that diagnosis rather than changing the entire
training method.

## 4. Experimental Conditions

The first study is a controlled comparison of three anchor weights:

| Condition | Self-play weight | Default-anchor weight | Anchor opening pairs per individual and generation |
| --- | ---: | ---: | ---: |
| Control | 1.00 | 0.00 | 0 |
| Small anchor | 0.90 | 0.10 | 1 |
| Moderate anchor | 0.80 | 0.20 | 2 |
| Disruptive pilot | 0.70 | 0.30 | 3 |

The control condition reproduces the zero-knowledge training objective in the
same code revision used for the comparison. Existing seed-1000 results are
historical evidence, not a substitute for this contemporaneous control.

The initial weights are development choices. They must be frozen before
production runs begin and must not be changed after inspecting confirmatory
results.

## 5. Anchored Fitness

The implementation exposes the condition through:

```text
--default-anchor-weight-percent N
--default-anchor-opening-pairs N
```

Both values default to zero. The small-anchor pilot uses `10` and `1`.
Ranking uses an exactly equivalent integer cross-product of the two normalized
score rates; floating-point arithmetic is not used for selection.

Checkpoint and experiment-report schema version 2 stores, for every ranked
individual and best-ever candidate:

- self-play half-points and available half-points;
- Default-anchor half-points and available half-points, when enabled;
- composite selection units and their maximum.

The composite can therefore be reconstructed from the raw components and
configuration. Version-1 zero-anchor checkpoints remain readable through an
explicit legacy mapping, while new documents no longer describe composite
selection units as `fitness_half_points`.

For each individual, calculate two normalized score rates:

```text
self_play_score = self-play half-points / available self-play half-points
anchor_score    = anchor half-points / available anchor half-points
```

The selection score is:

```text
selection_score =
    self_play_weight * self_play_score
  + anchor_weight    * anchor_score
```

The score must be represented without platform-dependent floating-point
ordering when it is used for ranking. Ties must be resolved by the existing
deterministic rules.

All individuals in the same anchored condition receive the same number of
anchor games. Anchor games do not alter Swiss pairings or replace self-play
games.

Each anchor opening pair consists of:

1. the evolved individual as White and `Default` as Black;
2. `Default` as White and the evolved individual as Black.

Both players use identical search depth, opening position, maximum game
length, and search implementation. The evaluation configuration is the only
intended player difference.

## 6. Training Configuration

Unless calibration forces a documented revision, retain the original
evolutionary configuration:

| Parameter | Value |
| --- | ---: |
| Generations | 100 |
| Population size | 32 |
| Swiss rounds per generation | 5 |
| Elite count | 2 |
| Parent candidate count | 3 |
| Gene mutation probability | 0.15 |
| Strong mutation probability | 0.02 |
| Mutation step | 0.10 |
| Strong mutation step | 0.50 |
| Training search depth | 4 |
| Maximum game length | 200 plies |
| Opening length | 4-10 plies |
| Maximum opening attempts | 100 |
| Workers | 16 |

One generation retains the original `160` self-play games. The additional
anchor cost is:

```text
small anchor:    32 individuals x 1 pair x 2 games =  64 games/generation
moderate anchor: 32 individuals x 2 pairs x 2 games = 128 games/generation
```

Therefore a 100-generation run contains:

| Condition | Self-play games | Anchor games | Total games |
| --- | ---: | ---: | ---: |
| Control | 16,000 | 0 | 16,000 |
| Small anchor | 16,000 | 6,400 | 22,400 |
| Moderate anchor | 16,000 | 12,800 | 28,800 |

## 7. Randomness and Reproducibility

All randomness must be derived from stable, recorded seeds. Training openings,
anchor openings, development benchmarks, and final validation must use
separate seed domains.

Within a generation and condition:

- every individual uses the same anchor opening set;
- colors are swapped within every opening pair;
- anchor openings change deterministically between generations;
- the same training master seed identifies corresponding runs across
  conditions;
- sequential and parallel execution must produce identical evolutionary
  state and checkpoints.

Seed derivation must include the condition identifier so that adding or
removing anchor games cannot perturb the existing self-play random stream.

## 8. Development Study

Implementation and calibration precede production. The development study
should:

1. verify deterministic ranking and checkpoint resumption;
2. measure the runtime cost of both anchored conditions;
3. run short matched-seed curves for the three conditions;
4. inspect generations `1`, `25`, `50`, `75`, and `100` when full pilot runs
   are justified;
5. select and freeze the production anchor condition before confirmatory
   validation.

Development may compare against `Default` and fixed `RandomGenome` ensembles.
It must use development seeds distinct from every final validation seed.

`RandomLegal` is excluded from training and primary analysis because the
zero-knowledge candidates already saturated that competence floor.

### 8.1 Small-anchor seed-1000 result

The 10% condition completed 100 generations for training seed `1000`.
Development comparisons used the same fixed Default openings, RandomLegal
openings, RandomGenome ensemble, and seeds as the zero-knowledge seed-1000
run.

The small anchor did not improve the mean Default score across generations
`1`, `25`, `50`, `75`, and `100`: both conditions averaged `21.00%`. The
anchored champion scored `22.50%` against Default versus `16.25%` for the
zero-knowledge champion, while retaining comparable RandomGenome performance.
The small benchmark and single training seed make that champion difference
exploratory.

The complete results and paired interpretation are recorded in
[`runs/small-anchor-seed-1000/DEVELOPMENT-BENCHMARKS.md`](runs/small-anchor-seed-1000/DEVELOPMENT-BENCHMARKS.md).

The next predefined development condition was the moderate 20% anchor. A
stronger 30% condition is registered separately as a disruptive pilot with an
explicit generation-50 decision gate; it must not be conflated with the
moderate condition or with a completed 100-generation result.

### 8.2 Disruptive 30% pilot

The disruptive pilot uses training seed `1000`, a 30% Default anchor, and
three opening pairs per individual and generation. It retains the frozen
100-generation objective for checkpoint compatibility, but the first tanda
stops at a valid generation-50 checkpoint. The generation-50 snapshot is
benchmarked against `Default` and the fixed `RandomGenome` ensemble before a
decision is made about resuming to generation 100.

The run plan is recorded in
[`runs/disruptive-30pct-seed-1000/EXPERIMENT.md`](runs/disruptive-30pct-seed-1000/EXPERIMENT.md).

## 9. Validation Strategy

Training against `Default` changes its scientific role. Performance against
`Default` remains an important target metric, but it is no longer an
independent held-out test of general playing strength.

Final validation should therefore report separate endpoints:

### 9.1 Target retention

Evaluate champions against `Default` on openings never used for training.
This measures whether the anchor achieved its explicit target and whether the
candidate can generalize to unseen positions against that target.

### 9.2 Knowledge-free distribution

Evaluate champions against a held-out `RandomGenome` ensemble generated with a
sealed seed. No member of this ensemble may be used during training or
development.

### 9.3 Cross-condition comparison

Use common held-out openings to play anchored champions against matched-seed
control champions. Report results per seed and in aggregate.

### 9.4 Independent external panel

Before production begins, define and seal at least one fixed reference panel
that is not used as an anchor. The panel specification, seeds, number of
openings, and success thresholds remain an open design decision and must be
frozen in this document before confirmatory runs start.

No single endpoint should silently stand in for all notions of strength.
Results against `Default`, random genomes, control champions, and the
independent panel must be presented separately.

## 10. Hypotheses

For the selected anchored condition:

### 10.1 Primary hypothesis

Anchored champions score higher against `Default` than matched-seed
zero-knowledge control champions on common held-out opening pairs.

### 10.2 Retention hypothesis

Anchored champions retain the improvement over the held-out `RandomGenome`
distribution demonstrated by the zero-knowledge training process.

### 10.3 Generalization hypothesis

Anchored champions do not regress against the independent external panel
relative to matched-seed zero-knowledge control champions.

Exact margins, sample counts, confidence intervals, and success criteria must
be frozen after calibration and before production.

## 11. Analysis

The evolutionary run, not an individual game, is the independent training
replicate. Opening pairs are the validation unit within a run.

Use matched training seeds across conditions. Analyze score differences on
common opening pairs and aggregate across runs with a hierarchical bootstrap:

1. resample matched training seeds;
2. within each sampled seed, resample complete opening pairs;
3. calculate the between-condition score difference;
4. construct a 95% confidence interval.

Do not compare Swiss fitness values across generations or conditions as if
they measured absolute playing strength. The anchored selection score is an
optimization objective, not a validation metric.

## 12. Artifacts

Store anchored artifacts separately:

```text
experiments/
  default-anchored-self-play/
    EXPERIMENT.md
    runs/
      <condition>-seed-<seed>/
        command.txt
        environment.json
        checkpoint.json
        report.json
        stdout.log
        stderr.log
```

Every run must record:

- condition and anchor weight;
- exact command and source revision;
- all seed values and derivation version;
- compiler version, build profile, workers, CPU model, and RAM;
- self-play and anchor game telemetry separately;
- checkpoint and versioned JSON report;
- start time, end time, and elapsed duration;
- stdout and stderr logs.

## 13. Execution Order

1. Implement anchored fitness behind explicit configuration.
2. Add unit, determinism, parallel-equivalence, and checkpoint tests.
3. Calibrate one generation for the small and moderate conditions.
4. Run matched-seed development pilots for control, small, and moderate
   conditions.
5. Select one anchor condition using only development results.
6. Define the independent external panel and freeze all production decisions.
7. Run the matched production repetitions.
8. Execute sealed validation and predefined statistical analysis.
9. Publish every run, including unsuccessful and incomplete runs.

## 14. Decisions Not Yet Frozen

The following items must be resolved before production:

- the selected anchor weight;
- development and production training seeds;
- anchor-opening seed domain;
- independent external reference panel;
- final-validation seeds and opening counts;
- number of matched production repetitions;
- primary effect-size threshold and confidence-interval criterion;
- whether runtime budget requires equal game counts or equal wall-clock
  comparisons as a secondary control.

Until these decisions are frozen, this document is a design protocol and all
anchored runs are exploratory.
