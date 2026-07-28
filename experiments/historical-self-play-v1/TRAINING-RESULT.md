# G50 Training Result

## Completion

The matched seed-2000 training batch completed successfully on 2026-07-28.
Both conditions used source revision
`55f914499cfe911cfcfc33954ccaf4502b2c92c9`, release mode, 16 workers, and the
frozen parameters in [`EXPERIMENT.md`](EXPERIMENT.md).

| Condition | Generations | Elapsed | Exit | Stderr | Checkpoint SHA-256 |
| --- | ---: | ---: | ---: | ---: | --- |
| Contemporary-only control | 50 | 39m 48.557s | 0 | 0 bytes | `BE52DD677DB018D04538C4046A4C7F31A11AA449F571EBE64DBE3FD6227AECCD` |
| 40% historical league | 50 | 1h 30m 21.144s | 0 | 0 bytes | `4C3311A1029B37A73D09DF047AE0C8A5642BFFB9BDE6E8BF6FAC736D07776612` |

The league required approximately 2.27 times the control wall-clock time.
Both checkpoints contain 50 generation records, a 32-individual final
population, and `state.next_generation = 50`. Both stdout logs end with
`Training complete: 50 generations; validation skipped`. No training process
remained after completion.

The launchers did not record the SHA-256 of the release executable or verify
its source revision. This is a minor provenance gap, documented in the main
experiment protocol and hardened for future runs with `prepare-and-run.ps1`.
No binary hash has been invented retroactively.

## Historical archive sanity checks

The control archive is empty. The league archive contains ten champions,
inserted at human generations G5, G10, G15, G20, G25, G30, G35, G40, G45,
and G50, matching the cadence of five and remaining below the maximum size of
16.

At G50:

- all 32 population members had distinct effective phenotypes;
- the archive grew from 9 to 10 entries;
- the sampled historical opponents represented G15, G20, G35, and G45;
- every candidate received the configured historical score capacity of 16
  half-points: four opponents times one color-swapped opening pair.

These checks establish that the historical objective was active and that its
telemetry is internally consistent. They are not evidence of external playing
strength.

## Interpretation boundary

No benchmark or validation was run as part of this task. Swiss or composite
training scores must not be compared across generations or conditions as a
strength measure.

The predefined next step is to evaluate the concrete G1, G25, and G50
snapshots, including retention against historical eras, the fixed
`RandomGenome` control, and the external `Default` reference. A decision about
continuing to G100 must wait for that analysis.
