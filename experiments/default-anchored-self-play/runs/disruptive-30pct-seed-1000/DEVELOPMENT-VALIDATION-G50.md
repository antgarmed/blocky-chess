# Disruptive 30% Pilot: Generation-50 Validation

The first pilot gate completed at a valid generation-50 checkpoint after two
training batches. This remains a development result, not sealed validation.

## Results

| Opponent | Candidate score | Score rate |
| --- | ---: | ---: |
| `Default` | 20 / 80 half-points | 25.00% |
| `RandomLegal` | 79 / 80 half-points | 98.75% |
| `RandomGenome` ensemble | 477 / 640 half-points | 74.53% |

The generation-50 candidate is individual `1487`.

## Comparison with the pilot's generation 25 gate

| Snapshot | Default | RandomGenome |
| --- | ---: | ---: |
| Generation 25 | 30.00% | 74.38% |
| Generation 50 | 25.00% | 74.53% |

The Default score declined by 5 percentage points from generation 25 to 50,
so the stronger anchor has not produced monotonic convergence toward
`Default`. However, generation 50 remains above both matched generation-50
development controls: 20.00% for zero-knowledge and 20.00% for the 10%
anchor. RandomGenome performance is essentially unchanged and remains well
above the initial random distribution.

## Gate interpretation

This is mixed evidence. The 30% anchor improves the generation-50 snapshot
relative to the previous conditions, but the generation-25 peak was not
maintained. Continuing to generation 100 is useful only as an exploratory
check of whether this late decline stabilizes or continues; the current data
do not support claiming convergence.

## Artifact hashes

```text
F1D6DE1932561D0EAD6330749136995052176EE7704866ECB062D680406E3D39  development-validation-generation-050-depth4.json
75C937A076830501D89347F2E8DD3463C4425B9F376F61E7AEE4F41834BB1AFC  control-benchmark-generation-050-depth4.json
```
