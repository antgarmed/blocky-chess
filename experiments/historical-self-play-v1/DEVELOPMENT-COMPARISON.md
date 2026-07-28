# G1/G25/G50 Development Comparison

## Frozen plan

This exploratory comparison evaluates the concrete G1, G25, and G50
generation champions from the matched seed-2000 control and 40% historical
league runs. It does not select `best-ever` and does not consume the sealed
final-validation seed.

The comparison reuses the established development panel so results remain
comparable with earlier experiments:

### Default reference

| Parameter | Value |
| --- | ---: |
| Search depth | 4 |
| Opening pairs | 20 |
| Games per candidate | 40 |
| Development seed | 2026072501 |
| Workers | 16 |
| Maximum game length | 200 plies |

### Random controls

| Parameter | Value |
| --- | ---: |
| Search depth | 4 |
| Opening pairs per opponent | 20 |
| RandomGenome opponents | 8 |
| RandomGenome games per candidate | 320 |
| RandomLegal games per candidate | 40 |
| Opening seed | 2026072502 |
| Opponent seed | 2026072503 |
| Workers | 16 |
| Maximum game length | 200 plies |

Every condition and generation uses the same openings and fixed random
opponents. Scores against `Default` and the RandomGenome ensemble are the
comparison endpoints. RandomLegal is retained only as a saturated competence
sanity check.

The G50 historical audit and archive are also inspected for internal
consistency and historical-era coverage. The current CLI does not expose a
standalone arbitrary-genome-versus-arbitrary-genome command, so this
comparison does not claim a complete round-robin retention test against every
archived champion.

## Decision guidance

Evidence in favor of the league requires:

- no important RandomGenome regression relative to the control;
- equal or better performance against `Default`;
- reasonable effective-phenotype diversity;
- an active, multi-era archive with non-degenerate historical scores.

With one training seed and 20 opening pairs, exact percentage differences are
exploratory rather than confirmatory.

## Results

The twelve predefined commands completed successfully in 14m 59.004s. The
comparison controller exited with code 0, produced every expected JSON report,
and left an empty stderr log.

Scores are percentages of the available half-points. G1 is identical because
the archive has not yet influenced evolution.

| Generation | Default control | Default league | Difference | RandomGenome control | RandomGenome league | Difference |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| G1 | 17.50% | 17.50% | 0.00 pp | 64.53% | 64.53% | 0.00 pp |
| G25 | 20.00% | 20.00% | 0.00 pp | 71.72% | 68.91% | -2.81 pp |
| G50 | 18.75% | 16.25% | -2.50 pp | 71.88% | 71.09% | -0.78 pp |
| Mean | 18.75% | 17.92% | -0.83 pp | 69.38% | 68.18% | -1.20 pp |

The concrete candidate IDs were:

| Generation | Control | League |
| --- | ---: | ---: |
| G1 | 21 | 21 |
| G25 | 735 | 730 |
| G50 | 1491 | 1479 |

RandomLegal remained saturated: the control scored 79/80 half-points at all
three snapshots; the league scored 79/80 at G1 and 78/80 at G25 and G50.

## Historical telemetry

The league archive contains ten distinct insertion points from G5 through
G50. At G50:

- all 32 current individuals had distinct effective phenotypes;
- the archive grew from 9 to 10 champions;
- the shared opponent panel sampled G15, G20, G35, and G45;
- the G50 selected champion scored 12/16 historical half-points (75%) against
  that sampled four-opponent panel;
- the G50 population historical scores ranged from 3/16 to 12/16.

This establishes that the memory objective was active, discriminative, and
did not collapse phenotype diversity. Because the panel is sampled as part of
training, its score is an optimization diagnostic rather than held-out
evidence.

## Interpretation

The 40% historical league does not meet the predefined G50 success guidance
in this single matched-seed development run:

- it regresses 2.50 percentage points against `Default` at G50;
- it regresses 0.78 percentage points against RandomGenome at G50;
- its G25 RandomGenome score is 2.81 percentage points below the control;
- its average across the three snapshots is lower on both external panels;
- it costs approximately 2.27 times as much wall-clock time as the control.

The RandomGenome difference at G50 is small enough to call broadly comparable
in isolation, but there is no compensating Default improvement. The result is
therefore neutral-to-negative, not evidence that this 40%/4-opponent/1-pair
league improves external strength by G50.

The run does show that historical pressure can be introduced without
phenotypic collapse and that the final selected candidate scores well against
the sampled training-era panel. A stronger retention claim would require a
separate fixed, held-out cross-era panel. The current CLI cannot execute that
panel directly, so it would be inappropriate to infer it from training
telemetry.

With the frozen rule stated in the experiment protocol, the evidence does not
justify automatically resuming both conditions to G100. Possible follow-up
experiments—such as a lower historical weight, fewer historical games, or a
fixed held-out cross-era benchmark—must be designed separately rather than
chosen post hoc from this comparison.

## Artifact hashes

```text
3CE65F0AAA1D49CC1A4F92F01952183BEA3E794A119711FB9688984BB7D495AE  runs/control-seed-2000/control-benchmark-generation-001-depth4.json
1E4A3C55AA88B6FDBBB7BE81421BB66A93CC10710D553FF28E9948CCB8BC1471  runs/control-seed-2000/control-benchmark-generation-025-depth4.json
3B172709685916AC355A86C6F25DA667DAA0F00D99168560B87F6710F7671410  runs/control-seed-2000/control-benchmark-generation-050-depth4.json
271912396BBC0A8154AF12E8C9F42CED2DB15119702057260D3EF77718DD1176  runs/control-seed-2000/development-validation-generation-001-depth4.json
D81E2307815BB66E5A917B647A6290FA022DEF67334E1F88419B26B54AF7FF93  runs/control-seed-2000/development-validation-generation-025-depth4.json
D070E9D94A9AE8A4B8FA7E225D78CF8A31419876C67DE497223D593E1B4F7F6F  runs/control-seed-2000/development-validation-generation-050-depth4.json
3CE65F0AAA1D49CC1A4F92F01952183BEA3E794A119711FB9688984BB7D495AE  runs/league-40pct-seed-2000/control-benchmark-generation-001-depth4.json
C0F2E1F1B84D4BA3B11B6CBB8FC690E2A53ECA297E6F816A93027C35784F0F96  runs/league-40pct-seed-2000/control-benchmark-generation-025-depth4.json
F644BFA45F20C812FAE7B8ED26F356CC5F0F6E67A437C00DF2F8CE79DC5FBE90  runs/league-40pct-seed-2000/control-benchmark-generation-050-depth4.json
271912396BBC0A8154AF12E8C9F42CED2DB15119702057260D3EF77718DD1176  runs/league-40pct-seed-2000/development-validation-generation-001-depth4.json
93233E6028F946CE01AA6DC3A649F8E481FD68443A4C19A4FBC1EB26499B5623  runs/league-40pct-seed-2000/development-validation-generation-025-depth4.json
3BFDEFDAE89B1A646E94A6EB9BC832C6B8F1CF15DA97AFA7C737BDB7E45136FD  runs/league-40pct-seed-2000/development-validation-generation-050-depth4.json
```
