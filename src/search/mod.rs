use shakmaty::{Chess, Move, MoveList};

use crate::utils::consts::MATE_VALUE;

pub type Value = i64;

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

    pub fn is_mate(&self) -> bool {
        self.get_mate_in().is_some()
    }

    pub fn get_mate_in(&self) -> Option<u64> {
        let diff = self.value.abs().abs_diff(MATE_VALUE.abs());

        if diff as i64 >= Self::MATE_VALUE_THRESHOLD {
            return None;
        }

        let mate_in = diff / 2;

        return Some(mate_in);
    }
}

pub struct SearchConfig {
    pub evaluation_function: fn(&Chess) -> Value,
    pub move_generator: fn(&Chess) -> MoveList,
}

pub trait SearchFactory {
    fn build(&self, config: SearchConfig) -> Box<dyn Search>;
}

pub trait Search {
    fn search(&self, initial_position: &Chess, depth: usize) -> SearchResult;
}

pub mod alphabeta;
