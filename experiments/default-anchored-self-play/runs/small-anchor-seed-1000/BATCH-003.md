# Training Batch 003

The third batch resumed generation 64. It completed and checkpointed
generations 65 through 69. Generation 70 finished computing, but checkpoint
replacement failed because Windows temporarily denied access to the existing
checkpoint (`os error 32`).

## Outcome

| Measurement | Result |
| --- | ---: |
| Initial generation | 64 |
| Final checkpointed generation | 69 |
| Computed but discarded generation | 70 |
| Elapsed wall-clock time | 474.316 s |
| Mean computed-generation time | 78.940 s |
| Mean reported throughput | 2.895 games/s |
| Aggregate Default-anchor score | 73 / 768 half-points |
| Standard error bytes | 304 |
| Stop reason | `checkpoint_write_failed` |

The authoritative checkpoint remains valid `blocky-evolution` format version
2 at generation 69. Deterministic resumption will recompute generation 70 from
that state.

The checkpoint's best-ever individual remains ID `1446`, with:

| Component | Score |
| --- | ---: |
| Self-play | 16 / 20 half-points |
| Default anchor | 2 / 4 half-points |
| Composite selection | 1,540 / 2,000 units |

The failed operation left a valid temporary generation-70 JSON document. It
was not accepted as authoritative because the atomic replacement did not
complete. Its hash was recorded before removing the disposable temporary file:

```text
7F31F224B78C031F8F63FBB2B240C95D1ED98157F55C8988CD28F005E6235668  .checkpoint.json.37060.tmp
```

The persistence implementation was subsequently changed to retry Windows
sharing and lock violations for up to two seconds. Other I/O errors still fail
immediately.

## Checkpoint integrity

```text
4C59EAC39630BF2D54517A2290568E648362751369721DA97848F5F3F6E69DAD  checkpoint.json
```
