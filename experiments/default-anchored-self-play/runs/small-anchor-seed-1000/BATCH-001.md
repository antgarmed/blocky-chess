# Training Batch 001

The first time-boxed training batch completed normally after approximately one
hour. The authoritative artifact is `checkpoint.json`.

## Outcome

| Measurement | Result |
| --- | ---: |
| Initial generation | 0 |
| Final completed generation | 48 |
| New generations | 48 |
| Elapsed wall-clock time | 3,602.789 s |
| Mean generation time | 75.007 s |
| Mean reported throughput | 3.073 games/s |
| Minimum reported throughput | 2.258 games/s |
| Maximum reported throughput | 4.326 games/s |
| Aggregate Default-anchor score | 821 / 6,144 half-points |
| Standard error bytes | 0 |
| Stop reason | `time_box_complete` |

The controller and `blocky-evolution` processes both terminated. The
checkpoint is valid `blocky-evolution` format version 2 and records:

- target generations: `100`;
- next generation: `48`;
- generation histories: `48`;
- population size: `32`;
- training seed: `1000`;
- Default-anchor weight: `10%`;
- Default-anchor opening pairs per individual and generation: `1`.

The checkpoint's best-ever individual at this point is ID `343`, with:

| Component | Score |
| --- | ---: |
| Self-play | 16 / 20 half-points |
| Default anchor | 0 / 4 half-points |
| Composite selection | 1,440 / 2,000 units |

This is a selection record, not a comparable cross-generation strength
measurement. In particular, a best-ever composite score with a zero anchor
component does not by itself determine whether the 10% anchor is effective;
that requires the predefined development benchmarks.

## Integrity

```text
B22A78C2F7B2E0ED41F22BB437646F44112017BFB4D4FD3FDC3B31EBD446B37B  checkpoint.json
```
