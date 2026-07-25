# Training Batch 002

The second batch resumed generation 48 and produced 16 complete generations
before both the controller and engine disappeared without writing the normal
controller status document. No engine error was recorded. The authoritative
artifact is the valid checkpoint at generation 64.

## Outcome

| Measurement | Result |
| --- | ---: |
| Initial generation | 48 |
| Final completed generation | 64 |
| New generations | 16 |
| Observed elapsed time through last checkpoint | 1,258.881 s |
| Mean generation time | 78.658 s |
| Mean reported throughput | 2.924 games/s |
| Aggregate Default-anchor score | 266 / 2,048 half-points |
| Standard error bytes | 0 |
| Stop reason | External or abrupt process termination |

The checkpoint is valid `blocky-evolution` format version 2 and preserves the
expected target, seed, population, anchor configuration, 64 generation
histories, and resumable population. The partial work after generation 64, if
any, was not checkpointed and is intentionally discarded.

The checkpoint's best-ever individual at this point is ID `1446`, with:

| Component | Score |
| --- | ---: |
| Self-play | 16 / 20 half-points |
| Default anchor | 2 / 4 half-points |
| Composite selection | 1,540 / 2,000 units |

The absence of `stderr` and of a controller-generated status prevents
attributing the interruption to an engine error. The run remains scientifically
valid because deterministic resumption starts from the last complete atomic
checkpoint.

## Integrity

```text
BDFFB79B1DB2B4E5930281F1F20085522B81BE9991704F9F40F82783DA6CD6C8  checkpoint.json
```
