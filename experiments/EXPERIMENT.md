# Evolutionary Evaluation Experiment

## 1. Purpose

This experiment evaluates whether the Blocky Chess evolutionary algorithm can
reliably produce evaluation parameters that outperform the engine's default
evaluation configuration.

The primary research question is:

> Does evolutionary training produce, reproducibly, an evaluation function
> that outperforms the default Blocky Chess evaluation under the same search
> budget and on openings that were not used during training?

The experiment is designed to distinguish between finding one successful
candidate and demonstrating that the evolutionary process works consistently
across independent runs.

## 2. Scope

The experiment compares:

- **Candidate:** the best-ever genome produced by one evolutionary run.
- **Reference:** the literal default `EvaluationConfig` used by Blocky Chess.

Both players use the same search implementation, search depth, opening
position, maximum game length, and hardware. The evaluation configuration is
the only intended difference between them.

This first experiment tests whether evolved champions outperform the default
engine. It does not, by itself, demonstrate that evolution is more efficient
than random search or another optimization algorithm. That question is
reserved for a separate control experiment described in Section 13.

## 3. Hypotheses

### 3.1 Primary hypothesis

At search depth 4, evolved champions score more than 50% against the default
evaluation on held-out opening pairs.

- **Null hypothesis (H0):** the expected candidate score is less than or equal
  to 50%.
- **Alternative hypothesis (H1):** the expected candidate score is greater
  than 50%.

### 3.2 Secondary hypothesis

At search depth 6, evolved champions do not suffer a practically relevant
regression against the default evaluation.

Depth 6 is treated as a generalization and non-inferiority check rather than as
a second primary superiority test.

## 4. Experimental Unit

The experimental unit is an **opening pair**, consisting of two games from the
same generated opening:

1. The candidate plays White and the reference plays Black.
2. The reference plays White and the candidate plays Black.

The two games are analyzed together because they share the same opening and
are not statistically independent. Swapping colors controls for first-move and
opening-color advantages.

Each game awards two half-points for a win, one half-point for a draw, and zero
half-points for a loss. A complete opening pair therefore distributes four
half-points between the candidate and the reference.

## 5. Independent Evolutionary Runs

Run the evolutionary algorithm **10 times**, using a different training master
seed for each run.

Proposed training seeds:

```text
1000, 1001, 1002, 1003, 1004,
1005, 1006, 1007, 1008, 1009
```

The number of independent runs is fixed before observing the final validation
results. Failed runs caused by infrastructure errors may be repeated with the
same seed. Runs must not be discarded or replaced because of poor chess
results.

The source revision, compiler version, build profile, command line, worker
count, CPU model, and operating system should be recorded for every run.

## 6. Training Configuration

The initial experiment uses the current default evolutionary hyperparameters:

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

One generation plays:

```text
population size x Swiss rounds = 32 x 5 = 160 games
```

One complete run therefore plays:

```text
160 games x 100 generations = 16,000 training games
```

Ten independent runs require 160,000 training games, excluding validation.

The initial population is knowledge-free and randomly generated. The default
evaluation is not inserted into the population and is used only as an external
reference during held-out validation.

## 7. Calibration Run

Before starting the 10 production runs, perform a calibration using the exact
production training configuration for one complete generation.

The calibration must measure:

- Wall-clock duration.
- Games per second.
- Mean and percentile game duration.
- Mean number of plies.
- Win, loss, and draw rates.
- Number and percentage of games adjudicated by `MaxPlies`.
- CPU utilization and effective worker count.

The calibration is operational rather than inferential: its results must not be
used as evidence for or against the research hypotheses.

If the estimated production cost is impractical, any reduction to population,
generations, rounds, depth, or number of repetitions must be documented here
before final validation results are observed.

## 8. Held-Out Validation

Each evolved champion is compared with the default evaluation using positions
generated from a validation seed that is independent from every training seed.

### 8.1 Validation set

Use:

- 200 held-out openings.
- Two color-swapped games per opening.
- The same held-out openings for every champion.
- The same openings at search depths 4 and 6.
- A maximum game length of 200 plies.
- Opening lengths from 4 to 10 plies.

This produces:

```text
200 openings x 2 colors x 2 depths = 800 validation games per champion
```

Across 10 champions, final validation requires 8,000 games.

Using a common validation set makes champion comparisons paired and reduces
opening-induced variance. The validation seed must be fixed before the final
experiment, recorded in every result, and never used for training or
hyperparameter selection.

The proposed fixed seed is the existing default validation seed:

```text
6215332838309450821
```

If this seed has already been repeatedly inspected during development, choose
and record a new sealed seed before the experiment instead.

### 8.2 Validation depths

- **Depth 4:** primary superiority endpoint.
- **Depth 6:** secondary generalization and non-inferiority endpoint.

The primary conclusion must not depend on combining both depths into a single
score.

## 9. Outcome Measures

### 9.1 Primary outcome

Candidate score rate at depth 4:

```text
candidate half-points / total available half-points
```

The result is calculated per opening pair and then aggregated. A score rate of
50% represents equal performance.

### 9.2 Secondary outcomes

- Candidate score rate at depth 6.
- Difference from 50% at each depth.
- 95% confidence interval for the score rate.
- Score-based Elo difference, reported with its confidence interval.
- Number and proportion of evolutionary runs whose champion scores above 50%.
- Distribution of champion genes across runs.
- Pairwise distance between champion genomes.
- Generation in which each best-ever champion first appeared.
- Training wall-clock time and throughput.
- Win, loss, and draw rates.
- Draw adjudication reasons, especially `MaxPlies`.

Elo is a derived presentation metric. The paired score rate and its uncertainty
remain the authoritative measurements.

## 10. Statistical Analysis

### 10.1 Within-run analysis

For each champion and depth:

1. Compute the candidate score for every opening pair.
2. Compute the mean candidate score rate.
3. Construct a 95% confidence interval by resampling complete opening pairs,
   never individual games.

A paired permutation or sign-flip test may be reported in addition to the
bootstrap confidence interval.

### 10.2 Across-run analysis

Aggregate results across the 10 independently trained champions using a
hierarchical bootstrap:

1. Resample evolutionary runs.
2. Within each sampled run, resample complete opening pairs.
3. Calculate the aggregate score rate.
4. Use the bootstrap distribution to construct a 95% confidence interval.

Use a deterministic and recorded analysis seed. The analysis implementation,
number of bootstrap samples, and software version must be recorded.

### 10.3 Multiplicity

Depth 4 is the only primary superiority endpoint. Depth 6, gene analysis,
convergence observations, and per-run results are secondary or exploratory.
This avoids treating several opportunities for success as one confirmatory
test.

## 11. Success Criteria

The primary experiment is considered successful when all of the following are
true:

1. The aggregate depth-4 score rate is greater than 50%.
2. The lower bound of its 95% hierarchical confidence interval is greater than
   50%.
3. At least 7 of the 10 independently evolved champions score above 50% at
   depth 4.
4. The depth-6 result shows no practically relevant regression.

For the secondary depth-6 check, the proposed non-inferiority threshold is 48%:
the lower bound of the 95% confidence interval should be greater than 48%.

The existing validation rule based on a minimum observed half-point margin may
still be included in reports for compatibility, but it is not the statistical
success criterion for this experiment.

## 12. Convergence and Interpretation

Swiss fitness values from different generations are not directly comparable.
Each generation can use different opponents and openings, so an increase in
the best recorded Swiss score does not by itself demonstrate convergence.

If convergence is studied, select the best individual from generations:

```text
0, 25, 50, 75, 99
```

Evaluate these snapshots against the same small development benchmark. This
benchmark must be separate from the sealed final validation set. Snapshot
validation is exploratory and must not influence which individual is declared
the run's champion; the best-ever training individual remains the champion
selected by the implemented algorithm.

## 13. Random-Search Control Experiment

A later experiment should determine whether the evolutionary operators add
value beyond evaluating many random genomes.

The control should:

- Generate random genomes from the same initial distribution.
- Use the same total number of game evaluations as evolution.
- Use independent seeds.
- Select candidates without crossover, mutation, or inherited elites.
- Validate the selected candidates on the same held-out opening pairs.
- Compare evolution and random search using paired results.

This control supports a stronger claim:

> Evolution is more effective than random search under an equal evaluation
> budget.

Without this control, the current experiment supports only the claim that the
evolutionary procedure can produce champions that outperform the default
evaluation.

## 14. Reproducibility and Artifacts

Store all experiment artifacts under a dedicated run directory:

```text
experiments/
  runs/
    <run-id>/
      command.txt
      environment.json
      checkpoint.json
      report.json
      stdout.log
      stderr.log
```

For every production run, retain:

- Exact command line.
- Git commit SHA and dirty-worktree status.
- Cargo and Rust compiler versions.
- Hardware and operating-system information.
- Training and validation seeds.
- Checkpoints.
- Complete JSON report.
- Standard output and error logs.
- Start time, end time, and elapsed duration.

The repository must be built in release mode. The worker count must be explicit
rather than relying on machine-dependent defaults.

Parallel and single-worker execution are expected to be deterministic. A
reproducibility audit should rerun at least one seed with a different worker
count and verify that the resulting report is identical apart from recorded
environment and timing metadata.

## 15. Planned Execution Order

1. Freeze this protocol and the experiment code.
2. Add any missing runtime, game-outcome, and analysis instrumentation.
3. Run the one-generation production calibration.
4. Confirm or revise the computational budget before seeing final results.
5. Freeze training seeds, validation seed, worker count, and source revision.
6. Run all 10 evolutionary training repetitions.
7. Validate all champions on the common held-out set.
8. Run the predefined statistical analysis.
9. Publish every run, including unsuccessful and incomplete runs.
10. Interpret the result according to the criteria in Section 11.

## 16. Decisions to Finalize After Calibration

The following decisions remain open until the production calibration is
complete:

- Whether 10 full repetitions at depth 4 fit the available compute budget.
- Whether 200 openings at depth 6 are affordable.
- The explicit worker count.
- The final sealed validation seed.
- The bootstrap sample count and analysis seed.
- Whether missing outcome and timing fields require changes to the JSON report.

These decisions must be finalized before inspecting the confirmatory validation
results.
