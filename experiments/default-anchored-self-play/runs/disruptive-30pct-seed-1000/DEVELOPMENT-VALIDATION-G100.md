# Disruptive 30% Pilot: Generation-100 Validation

The 30% Default-anchor pilot completed all 100 generations. This is a
development result using the same depth-4 openings and seeds as the previous
comparisons; it is not sealed final validation.

## Final results

| Opponent | Candidate score | Score rate |
| --- | ---: | ---: |
| `Default` | 14 / 80 half-points | 17.50% |
| `RandomLegal` | 78 / 80 half-points | 97.50% |
| `RandomGenome` ensemble | 479 / 640 half-points | 74.84% |

The generation-100 candidate is individual `2981`.

## 30% pilot curve

| Snapshot | Default | RandomGenome |
| --- | ---: | ---: |
| Generation 25 | 30.00% | 74.38% |
| Generation 50 | 25.00% | 74.53% |
| Generation 100 | 17.50% | 74.84% |

The Default score declined at every later checkpoint after the generation-25
peak. The final 17.50% is only 1.25 percentage points above the matched
generation-100 scores of the zero-knowledge and 10% conditions (both
16.25%), while RandomGenome performance stayed essentially flat.

## Conclusion

The 30% anchor produced a strong but transient improvement against `Default`;
it did not produce convergence or durable retention through generation 100.
The evolutionary process continued to improve or retain its performance over
the RandomGenome distribution, but the external Default alignment was not
preserved. This pilot therefore rejects the hypothesis that simply increasing
the anchor weight to 30% solves the late-run objective drift.

Any next intervention should address retention explicitly—for example, a
fixed training/selection panel kept separate from the sealed validation set,
a schedule for anchor weight, or a different objective—rather than treating
30% as a successful fixed-weight solution. The sealed validation set must
remain completely outside training and selection.

## Artifact hashes

```text
201B0F883FD1DEBDDD2805CE3C2AC2FFBC76D9BF9BB66A7042223337A69D7DAE  development-validation-generation-100-depth4.json
B1579D2AB9762457368909C79D462517F98C04848FC4F273DEA4415B948FE5AB  control-benchmark-generation-100-depth4.json
```
