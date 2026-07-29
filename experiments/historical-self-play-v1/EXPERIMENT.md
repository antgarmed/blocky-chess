# Historical Self-Play V1

## Purpose

This development experiment tests whether an internal league of champions
from earlier generations improves retention and reduces forgetting while
preserving strict zero-knowledge training.

No `EvaluationConfig::default()`, external engine, labelled position, opening
book, game database, or hand-authored opponent enters training. Historical
opponents are genomes produced by the same run from its random initial
population.

The primary question is:

> Does mixing contemporary self-play with games against historical champions
> improve retention and external playing strength relative to a matched
> contemporary-only control?

The experiment uses the new historical-league, tie-breaking, phenotype
deduplication, persistence, and telemetry code. Consequently, the earlier
zero-knowledge run is background evidence rather than a clean control.

## First batch: generation 1 through 50

Two matched conditions use source revision
`55f914499cfe911cfcfc33954ccaf4502b2c92c9` and training seed `2000`.

| Parameter | Control | Historical league |
| --- | ---: | ---: |
| Generations | 50 | 50 |
| Population | 32 | 32 |
| Swiss rounds | 5 | 5 |
| Elite count | 2 | 2 |
| Parent candidates | 3 | 3 |
| Gene mutation probability | 0.15 | 0.15 |
| Strong mutation probability | 0.02 | 0.02 |
| Mutation step | 0.10 | 0.10 |
| Strong mutation step | 0.50 | 0.50 |
| Search depth | 4 | 4 |
| Maximum game length | 200 plies | 200 plies |
| Opening length | 4-10 plies | 4-10 plies |
| Workers | 16 | 16 |
| Historical weight | 0% | 40% |
| Historical opponents | 0 | 4 |
| Historical opening pairs | 0 | 1 |
| Archive insertion cadence | disabled | 5 generations |
| Archive maximum size | 0 | 16 |

Contemporary Swiss self-play contributes 160 games per generation. Once the
archive can be sampled, the league adds 32 candidates x 4 opponents x 1
color-swapped pair = 256 historical games, for 416 total games per generation.
Champions first enter the archive at generation 5 and can affect selection
from generation 6.

The conditions run sequentially on the same host: control first, league
second. Training uses `--training-only` and `--checkpoint-every 1`; generation
50 is therefore a clean completed training target without final validation.
No continuation to generation 100 is authorized by this protocol.

## Predefined observations

Inspect the concrete generation snapshots G1, G25, and G50. A later
development-validation step must select snapshots with `--generation 1`,
`--generation 25`, and `--generation 50`, never silently substitute
`best-ever`.

At G50, evidence in favor of the league is:

- better retention against representatives from distinct historical eras;
- no important regression against the fixed `RandomGenome` control relative
  to the matched contemporary-only run;
- equal or better external result against `EvaluationConfig::default()`;
- reasonable effective-phenotype diversity;
- a G50 champion that performs against older representatives such as G5,
  G15, and G25, rather than only recent champions.

Swiss fitness is not comparable across generations and is not an external
strength metric.

## Reproducibility and artifacts

Each condition retains its command, environment, launcher, checkpoint,
per-run stdout and stderr logs, and completion status under:

```text
experiments/historical-self-play-v1/runs/
  control-seed-2000/
  league-20pct-seed-2000/
  league-40pct-seed-2000/
```

The authoritative artifact for this training-only batch is `checkpoint.json`.
A run is complete only when the launcher has exited successfully, stderr is
empty, the checkpoint is valid JSON, `state.next_generation` is 50, and
stdout records completion of training.

### Binary provenance

The completed seed-2000 launchers checked that
`target/release/blocky-evolution.exe` existed, but did not hash it or prove
that it was built from the recorded source revision. The checkpoints,
configuration, deterministic agreement at G1, logs, and observed behavior are
consistent with the intended code, so this does not invalidate the runs.
Nevertheless, exact binary provenance cannot be reconstructed retroactively
and remains a documented reproducibility limitation.

Future runs must be started through
[`prepare-and-run.ps1`](prepare-and-run.ps1). The preflight:

1. verifies that all workspace source paths match the expected source
   revision, while allowing later experiment-only documentation commits;
2. rejects untracked files under those source paths;
3. rebuilds `blocky-evolution` in release mode;
4. writes `binary-provenance.json` with the expected source revision, current
   HEAD, toolchain versions, binary length, and SHA-256;
5. only then invokes the run-specific launcher.

The run-specific launcher remains the authoritative record of runtime
arguments. Preflight provenance is additive and must never replace the exact
command, environment, logs, or checkpoint.

Launch a future run by detaching the preflight itself, passing an existing
run directory and the frozen source revision:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass `
  -File experiments/historical-self-play-v1/prepare-and-run.ps1 `
  -RunDirectory experiments/historical-self-play-v1/runs/<run-id> `
  -ExpectedSourceRevision <commit>
```

## Monitoring

## Retention panel

The example retention panel keeps the control champions at G5, G10, ..., G45
as the primary held-out panel. The historical-league panel starts at G10 and
continues through G45; those champions may have participated in the league's
training archive, so they are secondary and non-held-out. The combined result
is descriptive and must not replace the primary control-panel comparison.

The corrected example has 17 opponents, 10 opening pairs per opponent, 170
distinct opening positions, and 680 games for two candidates with both colors.

After each condition starts, verify process liveness, increasing CPU time,
expected stdout phase, empty stderr, and checkpoint validity:

1. first observation after approximately 3 minutes;
2. second observation after approximately 10 minutes from launch;
3. subsequent observations every approximately 20 minutes;
4. immediate observation on process exit or any detected error.

## Decision boundary

This task executes only the matched G50 training batch. Development
benchmarks and the decision whether to resume either condition to G100 are
separate follow-up work.
