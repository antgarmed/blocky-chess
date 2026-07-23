//! Deterministic simplified-Swiss scheduling.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use crate::openings::{OpeningId, StableRng};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IndividualId(pub u64);

/// Score represented in half-points to avoid floating-point ordering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score(pub u32);

impl Score {
    pub fn points(self) -> f64 {
        self.0 as f64 / 2.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Standing {
    pub individual: IndividualId,
    pub score: Score,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pairing {
    pub a: IndividualId,
    pub b: IndividualId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Round {
    pub number: usize,
    pub opening: OpeningId,
    pub pairings: Vec<Pairing>,
}

#[derive(Clone, Debug)]
pub struct SwissScheduler {
    seed: u64,
    participants: BTreeSet<IndividualId>,
    opponents: BTreeMap<IndividualId, BTreeSet<IndividualId>>,
    used_openings: BTreeSet<OpeningId>,
    rounds: usize,
}

impl SwissScheduler {
    pub fn new(
        participants: impl IntoIterator<Item = IndividualId>,
        seed: u64,
    ) -> Result<Self, PairingError> {
        let participants: BTreeSet<_> = participants.into_iter().collect();
        if participants.len() < 2 {
            return Err(PairingError::TooFewParticipants);
        }
        if participants.len() % 2 != 0 {
            return Err(PairingError::OddPopulation(participants.len()));
        }
        Ok(Self {
            seed,
            opponents: participants
                .iter()
                .map(|id| (*id, BTreeSet::new()))
                .collect(),
            participants,
            used_openings: BTreeSet::new(),
            rounds: 0,
        })
    }

    pub fn next_round(
        &mut self,
        standings: &[Standing],
        opening: OpeningId,
    ) -> Result<Round, PairingError> {
        validate_standings(&self.participants, standings)?;
        if self.used_openings.contains(&opening) {
            return Err(PairingError::OpeningAlreadyUsed(opening));
        }
        let pairings = if self.rounds == 0 {
            let mut ids: Vec<_> = self.participants.iter().copied().collect();
            let mut rng = StableRng::new(self.seed);
            for index in (1..ids.len()).rev() {
                ids.swap(index, rng.index(index + 1));
            }
            ids.chunks_exact(2)
                .map(|pair| Pairing {
                    a: pair[0],
                    b: pair[1],
                })
                .collect()
        } else {
            let scores: BTreeMap<_, _> = standings
                .iter()
                .map(|standing| (standing.individual, standing.score))
                .collect();
            let mut ids: Vec<_> = self.participants.iter().copied().collect();
            ids.sort_by_key(|id| (std::cmp::Reverse(scores[id]), *id));
            solve_pairings(&ids, &scores, &self.opponents)
                .ok_or(PairingError::NoNonRepeatingPairing { round: self.rounds })?
        };

        for pairing in &pairings {
            self.opponents
                .get_mut(&pairing.a)
                .unwrap()
                .insert(pairing.b);
            self.opponents
                .get_mut(&pairing.b)
                .unwrap()
                .insert(pairing.a);
        }
        let round = Round {
            number: self.rounds,
            opening,
            pairings,
        };
        self.used_openings.insert(opening);
        self.rounds += 1;
        Ok(round)
    }
}

fn solve_pairings(
    ids: &[IndividualId],
    scores: &BTreeMap<IndividualId, Score>,
    opponents: &BTreeMap<IndividualId, BTreeSet<IndividualId>>,
) -> Option<Vec<Pairing>> {
    if ids.is_empty() {
        return Some(Vec::new());
    }
    let a = ids[0];
    let mut candidates = ids[1..].to_vec();
    candidates.sort_by_key(|b| (scores[&a].0.abs_diff(scores[b].0), *b));
    for b in candidates {
        if opponents[&a].contains(&b) {
            continue;
        }
        let remaining: Vec<_> = ids
            .iter()
            .copied()
            .filter(|id| *id != a && *id != b)
            .collect();
        if let Some(mut rest) = solve_pairings(&remaining, scores, opponents) {
            rest.insert(0, Pairing { a, b });
            return Some(rest);
        }
    }
    None
}

fn validate_standings(
    participants: &BTreeSet<IndividualId>,
    standings: &[Standing],
) -> Result<(), PairingError> {
    let supplied: BTreeSet<_> = standings.iter().map(|s| s.individual).collect();
    if supplied.len() != standings.len() || &supplied != participants {
        return Err(PairingError::StandingsMismatch);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairingError {
    TooFewParticipants,
    OddPopulation(usize),
    StandingsMismatch,
    NoNonRepeatingPairing { round: usize },
    OpeningAlreadyUsed(OpeningId),
}

impl fmt::Display for PairingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for PairingError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn standings(count: u64) -> Vec<Standing> {
        (0..count)
            .map(|id| Standing {
                individual: IndividualId(id),
                score: Score(0),
            })
            .collect()
    }

    #[test]
    fn first_round_is_deterministic_and_covers_everyone_once() {
        let input = standings(8);
        let mut a = SwissScheduler::new(input.iter().map(|s| s.individual), 9).unwrap();
        let mut b = SwissScheduler::new(input.iter().map(|s| s.individual), 9).unwrap();
        let ra = a.next_round(&input, OpeningId(0)).unwrap();
        let rb = b.next_round(&input, OpeningId(0)).unwrap();
        assert_eq!(ra, rb);
        let ids: BTreeSet<_> = ra.pairings.iter().flat_map(|p| [p.a, p.b]).collect();
        assert_eq!(ids.len(), 8);
        assert!(ra.pairings.iter().all(|p| p.a != p.b));
    }

    #[test]
    fn odd_population_is_rejected() {
        assert!(matches!(
            SwissScheduler::new((0..3).map(IndividualId), 1),
            Err(PairingError::OddPopulation(3))
        ));
    }

    #[test]
    fn avoids_repeats_until_round_robin_is_exhausted_then_errors() {
        let input = standings(4);
        let mut scheduler = SwissScheduler::new(input.iter().map(|s| s.individual), 2).unwrap();
        let mut seen = BTreeSet::new();
        for round in 0..3 {
            let scheduled = scheduler.next_round(&input, OpeningId(round)).unwrap();
            for p in scheduled.pairings {
                let pair = [p.a, p.b].map(|id| id.0);
                assert!(seen.insert((pair[0].min(pair[1]), pair[0].max(pair[1]))));
            }
        }
        assert!(matches!(
            scheduler.next_round(&input, OpeningId(4)),
            Err(PairingError::NoNonRepeatingPairing { .. })
        ));
    }

    #[test]
    fn rejects_reusing_an_opening_in_a_later_round() {
        let input = standings(4);
        let mut scheduler = SwissScheduler::new(input.iter().map(|s| s.individual), 2).unwrap();
        scheduler.next_round(&input, OpeningId(7)).unwrap();

        assert_eq!(
            scheduler.next_round(&input, OpeningId(7)),
            Err(PairingError::OpeningAlreadyUsed(OpeningId(7)))
        );
    }

    #[test]
    fn later_round_prefers_near_scores_when_feasible() {
        let mut scheduler = SwissScheduler::new((0..4).map(IndividualId), 5).unwrap();
        let equal = standings(4);
        scheduler.next_round(&equal, OpeningId(0)).unwrap();
        let ranked = [
            Standing {
                individual: IndividualId(0),
                score: Score(6),
            },
            Standing {
                individual: IndividualId(1),
                score: Score(4),
            },
            Standing {
                individual: IndividualId(2),
                score: Score(2),
            },
            Standing {
                individual: IndividualId(3),
                score: Score(0),
            },
        ];
        let round = scheduler.next_round(&ranked, OpeningId(1)).unwrap();
        let total_gap: u32 = round
            .pairings
            .iter()
            .map(|p| {
                ranked[p.a.0 as usize]
                    .score
                    .0
                    .abs_diff(ranked[p.b.0 as usize].score.0)
            })
            .sum();
        assert!(total_gap <= 8);
    }

    #[test]
    fn backtracks_when_the_closest_local_choice_blocks_a_complete_matching() {
        let ids: Vec<_> = (0..4).map(IndividualId).collect();
        let scores = BTreeMap::from([
            (IndividualId(0), Score(6)),
            (IndividualId(1), Score(5)),
            (IndividualId(2), Score(2)),
            (IndividualId(3), Score(0)),
        ]);
        let opponents = BTreeMap::from([
            (IndividualId(0), BTreeSet::new()),
            (IndividualId(1), BTreeSet::new()),
            (IndividualId(2), BTreeSet::from([IndividualId(3)])),
            (IndividualId(3), BTreeSet::from([IndividualId(2)])),
        ]);

        let pairings = solve_pairings(&ids, &scores, &opponents).unwrap();

        assert!(pairings.contains(&Pairing {
            a: IndividualId(0),
            b: IndividualId(2)
        }));
        assert!(pairings.contains(&Pairing {
            a: IndividualId(1),
            b: IndividualId(3)
        }));
    }
}
