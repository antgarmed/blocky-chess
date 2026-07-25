# Small-Anchor Seed 1000 Development Benchmarks

## Purpose

These exploratory benchmarks compare the 10% Default-anchored seed-1000 run
with the zero-knowledge seed-1000 run on the same fixed opponents, openings,
search budget, and development seeds. They do not use the sealed final
validation seed.

Snapshots are the best individual from human-numbered generations `1`, `25`,
`50`, `75`, and `100`. `Best-ever` is the champion frozen by each training
objective.

## Configuration

### Default reference

| Parameter | Value |
| --- | ---: |
| Search depth | 4 |
| Opening pairs per candidate | 20 |
| Games per candidate | 40 |
| Development seed | 2026072501 |
| Workers | 16 |
| Maximum game length | 200 plies |

### Random controls

| Parameter | Value |
| --- | ---: |
| Search depth | 4 |
| Opening pairs per control | 20 |
| RandomLegal games per candidate | 40 |
| RandomGenome opponents | 8 |
| RandomGenome games per candidate | 320 |
| Benchmark opening seed | 2026072502 |
| RandomGenome seed | 2026072503 |
| Workers | 16 |
| Maximum game length | 200 plies |

The Default openings, random-control openings, and eight RandomGenome
opponents were verified identical to those used by the zero-knowledge
benchmarks.

## Anchored-run results

### Default reference

Scores use half-points. Every row distributes 80 half-points.

| Candidate | ID | Candidate | Default | W-D-L | Score |
| --- | ---: | ---: | ---: | ---: | ---: |
| Generation 1 | 10 | 18 | 62 | 3-12-25 | 22.50% |
| Generation 25 | 729 | 17 | 63 | 4-9-27 | 21.25% |
| Generation 50 | 1496 | 16 | 64 | 1-14-25 | 20.00% |
| Generation 75 | 2197 | 17 | 63 | 2-13-25 | 21.25% |
| Generation 100 | 2999 | 16 | 64 | 3-10-27 | 20.00% |
| Best-ever | 1446 | 18 | 62 | 2-14-24 | 22.50% |

All candidates remain below `Default`.

### Random controls

| Candidate | RandomLegal | W-D-L | RandomGenome ensemble | W-D-L | Ensemble score |
| --- | ---: | ---: | ---: | ---: | ---: |
| Generation 1 | 79-1 | 39-1-0 | 394-246 | 119-156-45 | 61.56% |
| Generation 25 | 78-2 | 38-2-0 | 466-174 | 167-132-21 | 72.81% |
| Generation 50 | 77-3 | 37-3-0 | 440-200 | 149-142-29 | 68.75% |
| Generation 75 | 77-3 | 37-3-0 | 465-175 | 163-139-18 | 72.66% |
| Generation 100 | 78-2 | 38-2-0 | 470-170 | 164-142-14 | 73.44% |
| Best-ever | 78-2 | 38-2-0 | 460-180 | 159-142-19 | 71.88% |

RandomLegal remains saturated and is not discriminative. The anchored run
retains a substantial advantage over the fixed RandomGenome ensemble.

## Paired comparison with zero-knowledge self-play

| Candidate | Default: zero | Default: anchored | Difference | RandomGenome: zero | RandomGenome: anchored | Difference |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Generation 1 | 22.50% | 22.50% | 0.00 pp | 61.56% | 61.56% | 0.00 pp |
| Generation 25 | 27.50% | 21.25% | -6.25 pp | 72.19% | 72.81% | +0.63 pp |
| Generation 50 | 20.00% | 20.00% | 0.00 pp | 72.97% | 68.75% | -4.22 pp |
| Generation 75 | 18.75% | 21.25% | +2.50 pp | 71.72% | 72.66% | +0.94 pp |
| Generation 100 | 16.25% | 20.00% | +3.75 pp | 74.53% | 73.44% | -1.09 pp |
| Best-ever | 16.25% | 22.50% | +6.25 pp | 71.09% | 71.88% | +0.78 pp |

Across the five predefined generation snapshots, both conditions average
exactly `21.00%` against `Default`. The anchored condition averages `69.84%`
against RandomGenome, compared with `70.59%` for zero-knowledge, a difference
of `-0.75` percentage points.

## Interpretation

The 10% anchor did not produce clear improvement against `Default` in this
single matched seed:

- it did not exceed the zero-knowledge peak of `27.50%` at generation 25;
- the mean Default score across the five snapshots is identical;
- its frozen champion improves from `16.25%` to `22.50%` relative to the
  zero-knowledge champion, but this is only five half-points on a small
  40-game development benchmark;
- the late anchored snapshots remain between `20.00%` and `21.25%`, while the
  zero-knowledge curve declines to `16.25%`. This is compatible with mild
  stabilization, but is not strong evidence by itself.

The useful self-play signal was retained. RandomGenome performance is broadly
similar between conditions, and generation 100 remains well above the initial
knowledge-free distribution.

The current evidence therefore supports:

> A 10% Default anchor with one opening pair per individual did not reliably
> realign evolution toward Default, although it may have reduced the late-run
> deterioration without materially sacrificing performance against random
> evaluation configurations.

The next development condition should be the predefined moderate anchor
(`20%`, two opening pairs per individual), rather than interpreting the small
anchor as successful. Conclusions remain exploratory because there is one
training seed and only 20 development opening pairs per candidate.

## Artifact hashes

```text
79408B2258648612589CC46BF7D906907AF7F879EB8C1F297057ED3CC8D8F502  development-validation-best-ever-depth4.json
FFD641EAB1ED50B14106293AD34496AE74672176420F4160E9A43E69FC42E05E  development-validation-generation-001-depth4.json
BBD8B9D96E92F0BFA8289AB6D8D398D93B0495C23574189524FBD4C1A3897DD9  development-validation-generation-025-depth4.json
4980F7A42B780623BB748DD27BF9096454FD9E3EB267C2EB01BE7353CB426B64  development-validation-generation-050-depth4.json
DCA185A2161BD8EA47FB2B6341C655D8B94FD80834C2B1C1C6B2DD0692152C16  development-validation-generation-075-depth4.json
4C056B3DDD37FB5292EBCDCC8950A7D082630FF133520DBEDAB404EA64353D86  development-validation-generation-100-depth4.json
F4EBBCE7C9634E965ACBC9903D1637B8A7C5C5B8D3B2CE76D135837F5F403716  control-benchmark-best-ever-depth4.json
122D2AE0968406F9C2F3A4E26A2467D844FEC9E70721EFE5437FE874319EA9CE  control-benchmark-generation-001-depth4.json
3587C508B85F2DB7E1C12E0FBA5BD01E27DFBE1609DCBCD186AB0A07E0B8CE63  control-benchmark-generation-025-depth4.json
31495D49B86710D114FC778759A1511B467DA6DAF2F3FA512BDE0750BD82D652  control-benchmark-generation-050-depth4.json
1761A2092B60CF79B18DF4DC151FFFE8BA1F914627BBED647540D194894A2240  control-benchmark-generation-075-depth4.json
10FCBF6402784C65F2A4DCE41AFDE67D932E8A1AAA6F83C3F9746A83017FA8B5  control-benchmark-generation-100-depth4.json
```
