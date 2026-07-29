# 20% Historical League: G50 Results

## Scope

`league-20pct-seed-2000` used training seed `2000`, the historical-self-play-v1
code path, four historical opponents, one color-swapped opening pair per
opponent, insertion every five generations, archive maximum 16, and no
`best-ever` selector. Training stopped at G50 and did not continue to G100.
The control checkpoint was reused; no control training was run.

## Concrete G1/G25/G50 validation

The new league champions were selected explicitly by generation. The same
depth-4, 20-opening panels and seeds as the earlier comparison were used:
Default validation seed `2026072501`; RandomGenome benchmark seed `2026072502`
and opponent seed `2026072503`; 16 workers; 200-plies maximum.

| Generation | Champion ID | Default | RandomGenome | RandomLegal sanity check |
| ---: | ---: | ---: | ---: | ---: |
| G1 | 21 | 17.50% | 64.53% | 98.75% |
| G25 | 742 | 18.75% | 72.66% | 98.75% |
| G50 | 1486 | 23.75% | 73.28% | 97.50% |

The three-snapshot mean is 20.00% against Default and 70.16% against the
RandomGenome ensemble. These are exploratory single-seed results, not a
confirmatory strength estimate.

## Fixed retention benchmark

The fixed panel used the Control G5, G10, ..., G45 champions as opponents,
with 10 opening pairs per opponent, depth 4, 16 workers, and retention seed
`2026072901`. Each candidate played 180 games / 360 half-points.

| Candidate | Score | Win/Draw/Loss | Retention |
| --- | ---: | ---: | ---: |
| Control 0%, G50 | 175/360 | 43/89/48 | 48.61% |
| League 20%, G50 | 195/360 | 54/87/39 | 54.17% |
| League 40%, G50 | 195/360 | 50/95/35 | 54.17% |

On this fixed primary control panel, the 20% league improves on the control by
5.56 percentage points and ties the 40% league. This panel measures retention
against sampled control-era champions; it is not a claim of independent
external strength.

## Brief comparison: strength, retention, cost

The 20% league is the strongest of the three conditions on the fixed external
development panels at G50 in this batch: 23.75% versus Default and 73.28%
versus RandomGenome, compared with the earlier 40% result of 16.25% and
71.09%, respectively, and the control result of 18.75% and 71.88%. Across
G1/G25/G50, the 20% league averages 20.00%/70.16%, versus the control's
18.75%/69.38% and the 40% league's 17.92%/68.18%.

Retention is also better than control (54.17% vs 48.61%) and equal to 40% on
the fixed primary panel. The cost is 4,485.073 seconds (1h14m45.073s), about
1.88x the 0% control's 2,388.557 seconds and 0.83x the 40% run's 5,421.144
seconds. Within this one seed, 20% is the best trade-off, but the result is
still exploratory and does not authorize continuation to G100.

## Provenance and hashes

Training checkpoint SHA-256:
`4EC1430331EC355765E95C49780C249FF62ADC2335376832076FD11F56D77309`.
Release binary SHA-256:
`2CF8C8666FEE7949301874A83AD54C8668DBE2785A9BC8965DA3431EF925D9D4`.

| Artifact | SHA-256 |
| --- | --- |
| Retention report | `72C06BD40F7B72E4C1B98B4D4ECB8AD89F3C1D2E2967CD737E97E39135B281A3` |
| 20% validation G1 | `271912396BBC0A8154AF12E8C9F42CED2DB15119702057260D3EF77718DD1176` |
| 20% validation G25 | `0BB480F6855C333005E22459D354A8F43D9880BA27FA325C02DE809546A1C360` |
| 20% validation G50 | `B1B66C4F182F9ECD7FCEE3EF8A03420BBD567AC28754BED6752DCA2C8EFA7BD8` |
| 20% benchmark G1 | `3CE65F0AAA1D49CC1A4F92F01952183BEA3E794A119711FB9688984BB7D495AE` |
| 20% benchmark G25 | `4098C4ADE10CC8ABD482BDD344DC9E93E3603345D301C62EC64A4D739AC8B480` |
| 20% benchmark G50 | `F5ECAE9E5439BDD5FFD68FDA96C0088C2FBFF1FDF4C499C048684151F985C849` |

The full commands, environment, launcher, status, stdout/stderr, checkpoint,
validation reports, retention manifest, retention logs, and retention status
are retained alongside this report.
