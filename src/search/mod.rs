use shakmaty::{Chess, Move, MoveList};
use std::sync::atomic::AtomicBool;

use crate::utils::consts::MATE_VALUE;

pub type Value = i64;

#[derive(Clone)]
pub struct SearchResult {
    pub value: Value,
    pub principal_variation: Vec<Move>,
}

impl SearchResult {
    const MATE_VALUE_THRESHOLD: Value = 100;

    pub fn is_white_winning(&self) -> bool {
        self.value > 0
    }

    pub fn is_black_winning(&self) -> bool {
        self.value < 0
    }

    pub fn get_mate_in(&self) -> Option<u64> {
        let diff = self.value.abs().abs_diff(MATE_VALUE.abs());

        if diff as i64 >= Self::MATE_VALUE_THRESHOLD {
            return None;
        }

        let mate_in = (diff + 1) / 2;

        Some(mate_in)
    }
}

#[derive(Clone)]
pub struct SearchConfig {
    pub evaluation_function: fn(&Chess) -> Value,
    pub move_generator: fn(&Chess) -> MoveList,
}

pub trait Search: Send + Sync {
    fn search_with_stop(
        &self,
        initial_position: &Chess,
        depth: usize,
        stop: &AtomicBool,
    ) -> Option<(usize, SearchResult)>;
}

pub mod alpha_beta_iterative_deepening;
pub mod alphabeta;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_mate_in_rounds_up_for_a_one_ply_mate() {
        let result = SearchResult {
            value: MATE_VALUE - 1,
            principal_variation: Vec::new(),
        };

        assert_eq!(result.get_mate_in(), Some(1));
    }
}
