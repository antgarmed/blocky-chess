# Disruptive 30% Default-Anchor Pilot

## Purpose

This is an exploratory development pilot for a stronger Default anchor. It
tests whether a materially larger absolute signal can improve retention
against `EvaluationConfig::default()` without collapsing the useful
knowledge-free self-play signal.

The condition is intentionally more disruptive than the predefined 20%
moderate condition. It is a decision gate, not a confirmatory result.

## Frozen condition

| Parameter | Value |
| --- | ---: |
| Training seed | 1000 |
| Generations target | 100 |
| Pilot stopping point | 50 |
| Population | 32 |
| Swiss rounds | 5 |
| Search depth | 4 |
| Workers | 16 |
| Default-anchor weight | 30% |
| Default-anchor opening pairs | 3 |
| Self-play games per generation | 160 |
| Anchor games per generation | 192 |
| Total games per generation | 352 |
| Maximum game length | 200 plies |

The target remains 100 generations for checkpoint compatibility. The first
tanda stops only after a valid checkpoint has accepted generation 50; it is
not a completed 100-generation experiment.

## Comparators

At the generation-50 gate, use the already frozen development benchmarks and
the same depth-4 conditions as the 10% anchored and zero-knowledge runs:

- `Default` on the common development openings;
- the fixed `RandomGenome` ensemble;
- `RandomLegal` only as a saturated sanity check.

The decision must use the generation-50 snapshot, not the best-ever candidate.
The best-ever result may be reported separately as exploratory context.

## Decision rule

Continue from generation 50 to 100 only if the 30% condition shows a clear
improvement against `Default` relative to both the zero-knowledge and 10%
anchor generation-50 snapshots, while retaining broadly comparable
`RandomGenome` performance. A large Default gain accompanied by a severe
RandomGenome regression is evidence of over-anchoring, not success.

If the gate is not met, preserve the checkpoint and logs as a complete
intermediate observation and do not silently reinterpret it as a failed
100-generation run.

## Operational plan

1. Run a one-generation calibration and record its throughput.
2. Launch the versioned batch controller with `--generations 100`,
   `--checkpoint-every 1`, and the 30% anchor arguments.
3. Stop at the first valid checkpoint at or beyond generation 50.
4. Verify process exit, empty stderr, valid checkpoint JSON, and no final
   validation report.
5. Run the matched development benchmarks and write the generation-50
   comparison before deciding whether to resume.

All artifacts belong under this directory. This pilot uses development seeds
only and must not consume the sealed final-validation seed.
