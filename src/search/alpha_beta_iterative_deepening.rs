use shakmaty::Chess;
use std::sync::atomic::AtomicBool;

use super::{alphabeta::AlphaBetaSearch, Search, SearchConfig, SearchResult};

pub struct AlphaBetaIterativeDeepeningSearch {
    alpha_beta_search: AlphaBetaSearch,
}

impl AlphaBetaIterativeDeepeningSearch {
    pub fn new(config: SearchConfig) -> Self {
        Self {
            alpha_beta_search: AlphaBetaSearch::new(config),
        }
    }
}

impl Search for AlphaBetaIterativeDeepeningSearch {
    fn search_with_stop(
        &self,
        initial_position: &Chess,
        depth: usize,
        stop: &AtomicBool,
    ) -> Option<(usize, SearchResult)> {
        let mut completed = None;
        for d in 1..=depth {
            match self
                .alpha_beta_search
                .search_with_stop(initial_position, d, stop)
            {
                Some(result) => completed = Some(result),
                None => break,
            }
        }
        completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::basic_movegen::basic_movegen;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn zero(_: &Chess) -> i64 {
        0
    }

    #[test]
    fn cancelled_search_does_not_publish_an_incomplete_iteration() {
        let search = AlphaBetaIterativeDeepeningSearch::new(SearchConfig {
            evaluation_function: zero,
            move_generator: basic_movegen,
        });
        let stop = AtomicBool::new(true);
        assert!(search
            .search_with_stop(&Chess::default(), 4, &stop)
            .is_none());
        assert!(stop.load(Ordering::Relaxed));
    }
}
