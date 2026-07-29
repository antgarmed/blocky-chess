# 20% League G50 Training Result

The training-only seed-2000 batch completed successfully on 2026-07-29.
The run used release mode, 16 workers, the historical-self-play-v1
hyperparameters, and changed only the historical weight from the matched
40% condition to 20%. The control was reused from `control-seed-2000`.

| Item | Result |
| --- | --- |
| Generations | 50 |
| Population | 32 |
| Elapsed | 1h 14m 45.073s |
| Exit code | 0 |
| Stderr | 0 bytes |
| Checkpoint `state.next_generation` | 50 |
| Checkpoint SHA-256 | `4EC1430331EC355765E95C49780C249FF62ADC2335376832076FD11F56D77309` |
| Binary SHA-256 | `2CF8C8666FEE7949301874A83AD54C8668DBE2785A9BC8965DA3431EF925D9D4` |

The historical configuration was weight 20%, 4 opponents, 1 opening pair per
opponent, insertion cadence 5, and maximum archive size 16. The archive has
10 entries at G50, inserted at G5 through G50, and the final population has 32
members. Stdout ends with `Training complete: 50 generations; validation
skipped`; no validation or `best-ever` selection was used.

The current source revision is recorded in `environment.json` and
`binary-provenance.json`. It includes the later retention-benchmark tooling
needed by this task; no training algorithm or non-historical hyperparameter
was changed for this run.
