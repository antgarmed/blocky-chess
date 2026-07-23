use shakmaty::{Chess, Move, MoveList};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use crate::evaluation::EvaluationConfig;
use crate::utils::consts::MATE_VALUE;
use std::sync::{Arc, RwLock};

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

        let mate_in = diff.div_ceil(2);

        Some(mate_in)
    }
}

#[derive(Clone)]
pub struct SearchConfig {
    pub evaluation_function: fn(&Chess, &EvaluationConfig) -> Value,
    pub move_generator: fn(&Chess) -> MoveList,
    pub evaluation_config: Arc<RwLock<EvaluationConfig>>,
}

pub struct SearchLimits<'a> {
    pub depth: Option<usize>,
    pub deadline: Option<Instant>,
    pub stop: &'a AtomicBool,
}

impl SearchLimits<'_> {
    pub fn should_stop(&self) -> bool {
        self.stop.load(std::sync::atomic::Ordering::Relaxed)
            || self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
    }
}

pub trait Search: Send + Sync {
    fn set_evaluation_config(&self, config: EvaluationConfig);

    fn search_with_limits(
        &self,
        initial_position: &Chess,
        limits: &SearchLimits<'_>,
        on_iteration: &mut dyn FnMut(usize, &SearchResult),
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
