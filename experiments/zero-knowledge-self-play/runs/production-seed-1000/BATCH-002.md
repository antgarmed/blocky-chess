# Batch 002

## Result

- resumed generation: 77;
- completed generation: 100;
- generations completed in this batch: 23;
- training games completed in this batch: 3,680;
- elapsed time: 1,277.539 seconds (21 minutes 17.539 seconds);
- termination reason: training complete;
- standard error bytes: 0;
- validation started: no;
- report generated: no.

The batch used `--training-only`, reached the configured target of 100
generations, atomically wrote the final checkpoint, and exited naturally. The
last output line was:

```text
Training complete: 100 generations; validation skipped
```

The sealed validation seed was not executed.

## Final training state

- checkpoint format: `blocky-evolution` version 1;
- generation records: 100;
- population: 32;
- training seed: 1000;
- total training games: 16,000;
- frozen champion ID: 845;
- champion training fitness when selected: 16 half-points (8 points);
- champion first appeared in zero-based generation 28 (human generation 29).

Swiss fitness is opponent- and opening-dependent and must not be interpreted
as a validation score or compared as a convergence curve across generations.

## Artifact integrity

- `checkpoint.json` SHA-256:
  `CA13C0B0371F5C48853F3FD3B99F86AE796AC7EB0D8E0BC94F5DF3D1B89CBEFC`
- `batch-002-stdout.log` SHA-256:
  `BCB5AC80DC8E5602AFFDF5B52408DF1AA5FBE65EBB1011DBAABD05F6BC2CC65A`
- `batch-002-stderr.log` SHA-256:
  `E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855`
