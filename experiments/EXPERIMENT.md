# Evolution Experiment Portfolio

The evolutionary work is organized as two separate experiments so that a
change in the training objective cannot be confused with a continuation of the
original protocol.

## Experiments

| Experiment | Training signal | Status |
| --- | --- | --- |
| [`zero-knowledge-self-play`](zero-knowledge-self-play/EXPERIMENT.md) | Contemporary self-play only | Original protocol; seed 1000 completed |
| [`default-anchored-self-play`](default-anchored-self-play/EXPERIMENT.md) | Contemporary self-play plus a `Default` anchor | Small-anchor seed 1000 completed; disruptive 30% pilot established |
| [`historical-self-play-v1`](historical-self-play-v1/EXPERIMENT.md) | Contemporary self-play with or without an internal historical league | Matched seed-2000 G50 development comparison running |

The original experiment and all of its artifacts remain authoritative for the
zero-knowledge condition. Results from the anchored experiment must not be
pooled with it as if they came from the same training procedure.

## Directory convention

```text
experiments/
  EXPERIMENT.md
  zero-knowledge-self-play/
    EXPERIMENT.md
    runs/
      <run-id>/
  default-anchored-self-play/
    EXPERIMENT.md
    runs/
      <run-id>/
  historical-self-play-v1/
    EXPERIMENT.md
    runs/
      <run-id>/
```

Every run directory should retain its exact command, environment, source
revision, checkpoint, report, logs, and human-readable result notes.
