# Disruptive 30% Pilot: Generation-25 Informal Validation

This is an intermediate development check after the first tanda stopped at a
valid generation-30 checkpoint. It is not a sealed final validation and does
not justify treating the pilot as a completed 100-generation experiment.

## Configuration

The generation-25 candidate was evaluated at search depth 4 using the same
development seeds and 20 opening pairs used by the previous zero-knowledge
and 10% anchor comparisons:

- `Default`: 40 games, development seed `2026072501`;
- `RandomGenome`: 8 opponents x 40 games, opponent seed `2026072503`;
- `RandomLegal`: 40 games, as a saturated sanity check.

## Results

| Opponent | Candidate score | W-D-L / controls | Score rate |
| --- | ---: | ---: | ---: |
| `Default` | 24 / 80 half-points | 10-20-10 | 30.00% |
| `RandomLegal` | 75 / 80 half-points | 37-1-2 | 93.75% |
| `RandomGenome` ensemble | 476 / 640 half-points | see JSON telemetry | 74.38% |

## Matched generation-25 comparison

| Condition | Default | RandomGenome |
| --- | ---: | ---: |
| Zero-knowledge | 27.50% | 72.19% |
| Small anchor, 10% | 21.25% | 72.81% |
| Disruptive anchor, 30% | **30.00%** | **74.38%** |

The 30% pilot is currently ahead on both development endpoints at this
snapshot: +2.50 percentage points against `Default` versus zero-knowledge,
+8.75 points versus the 10% anchor, and +2.19 points against `RandomGenome`
versus zero-knowledge.

## Interpretation and gate status

This is the first indication that a stronger anchor may materially realign
the objective. It is still only one 40-game Default sample at one generation,
so sampling noise and snapshot selection remain substantial. The result is
strong enough to justify continuing the pilot to generation 50, but not to
skip that gate or claim convergence.

## Artifact hashes

```text
1BEECB2670CFB9347630F2AC3880F7BA4B4B32BAB77B857538523086B67DD4F5  development-validation-generation-025-depth4.json
10A8D58E556090069A33DF36E064F46B0EA156C25C0FA44A4E44824D73B5D319  control-benchmark-generation-025-depth4.json
```
