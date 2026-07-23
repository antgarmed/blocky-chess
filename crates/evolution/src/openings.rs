//! Reproducible, knowledge-free opening generation.

use std::{collections::HashSet, error::Error, fmt};

use shakmaty::{zobrist::Zobrist128, Chess, EnPassantMode, Move, Position};

use crate::training::TrainingConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OpeningId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opening {
    pub id: OpeningId,
    pub seed: u64,
    pub moves: Vec<Move>,
    pub position: Chess,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpeningPool {
    openings: Vec<Opening>,
}

impl OpeningPool {
    pub fn generate(count: usize, config: &TrainingConfig) -> Result<Self, OpeningGenerationError> {
        let mut openings = Vec::with_capacity(count);
        let mut positions = HashSet::with_capacity(count);

        for index in 0..count {
            let mut accepted = None;
            for attempt in 0..config.max_opening_attempts() {
                let seed = derive_seed(config.master_seed(), index as u64, attempt as u64);
                let opening = generate_one(OpeningId(index as u64), seed, config);
                if is_non_terminal(&opening.position)
                    && positions.insert(position_key(&opening.position))
                {
                    accepted = Some(opening);
                    break;
                }
            }
            openings.push(accepted.ok_or(OpeningGenerationError::AttemptsExhausted {
                opening: OpeningId(index as u64),
                attempts: config.max_opening_attempts(),
            })?);
        }
        Ok(Self { openings })
    }

    pub fn openings(&self) -> &[Opening] {
        &self.openings
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpeningGenerationError {
    AttemptsExhausted { opening: OpeningId, attempts: usize },
}

impl fmt::Display for OpeningGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AttemptsExhausted { opening, attempts } => write!(
                f,
                "could not generate unique non-terminal opening {opening:?} in {attempts} attempts"
            ),
        }
    }
}

impl Error for OpeningGenerationError {}

fn generate_one(id: OpeningId, seed: u64, config: &TrainingConfig) -> Opening {
    let mut rng = StableRng::new(seed);
    let start = *config.opening_plies().start();
    let end = *config.opening_plies().end();
    let first_even = start + start % 2;
    let even_count = (end - first_even) / 2 + 1;
    let target = first_even + 2 * rng.index(even_count);
    let mut position = Chess::default();
    let mut moves = Vec::with_capacity(target);
    for _ in 0..target {
        let legal = position.legal_moves();
        if legal.is_empty() {
            break;
        }
        let selected = legal[rng.index(legal.len())];
        position.play_unchecked(selected);
        moves.push(selected);
    }
    Opening {
        id,
        seed,
        moves,
        position,
    }
}

fn is_non_terminal(position: &Chess) -> bool {
    !position.legal_moves().is_empty()
        && !position.is_insufficient_material()
        && position.halfmoves() < 100
}

fn position_key(position: &Chess) -> u128 {
    position.zobrist_hash::<Zobrist128>(EnPassantMode::Legal).0
}

pub(crate) fn derive_seed(master: u64, stream: u64, attempt: u64) -> u64 {
    let mut value = master
        ^ stream.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ attempt.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

/// Small explicit SplitMix64 stream. Its algorithm is fixed here, so results
/// do not depend on a platform RNG or a dependency changing implementation.
pub(crate) struct StableRng(u64);

impl StableRng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        derive_seed(self.0, 0, 0)
    }

    pub(crate) fn index(&mut self, length: usize) -> usize {
        debug_assert!(length > 0);
        // Rejection avoids modulo bias while retaining a fully stable stream.
        let threshold = u64::MAX - u64::MAX % length as u64;
        loop {
            let value = self.next();
            if value < threshold {
                return (value % length as u64) as usize;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(seed: u64) -> TrainingConfig {
        TrainingConfig::new(2, 100, seed, 4..=10, 100).unwrap()
    }

    #[test]
    fn same_seed_reproduces_moves_and_positions() {
        let a = OpeningPool::generate(8, &config(42)).unwrap();
        let b = OpeningPool::generate(8, &config(42)).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_produce_diversity() {
        let a = OpeningPool::generate(8, &config(42)).unwrap();
        let b = OpeningPool::generate(8, &config(43)).unwrap();
        assert!(a
            .openings()
            .iter()
            .zip(b.openings())
            .any(|(left, right)| left.moves != right.moves || left.position != right.position));
    }

    #[test]
    fn openings_replay_legally_are_even_and_non_terminal_and_unique() {
        let config = config(7);
        let pool = OpeningPool::generate(24, &config).unwrap();
        let mut seen = HashSet::new();
        for opening in pool.openings() {
            let mut position = Chess::default();
            for &mv in &opening.moves {
                assert!(position.is_legal(mv));
                position = position.play(mv).unwrap();
            }
            assert_eq!(position, opening.position);
            assert_eq!(opening.moves.len() % 2, 0);
            assert!(config.opening_plies().contains(&opening.moves.len()));
            assert!(is_non_terminal(&opening.position));
            assert!(seen.insert(position_key(&opening.position)));
        }
    }

    #[test]
    fn exhaustion_is_typed() {
        let config = TrainingConfig::new(1, 1, 1, 0..=0, 1).unwrap();
        assert!(matches!(
            OpeningPool::generate(2, &config),
            Err(OpeningGenerationError::AttemptsExhausted {
                opening: OpeningId(1),
                attempts: 1
            })
        ));
    }
}
