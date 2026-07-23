use super::{alphabeta::AlphaBetaSearch, Search, SearchConfig, SearchLimits, SearchResult};
use shakmaty::Chess;

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
    fn search_with_limits(
        &self,
        initial_position: &Chess,
        limits: &SearchLimits<'_>,
        on_iteration: &mut dyn FnMut(usize, &SearchResult),
    ) -> Option<(usize, SearchResult)> {
        let mut completed = None;
        let mut d = 1;
        while limits.depth.is_none_or(|max_depth| d <= max_depth) {
            match self
                .alpha_beta_search
                .search_depth_with_limits(initial_position, d, limits)
            {
                Some((_, result)) => {
                    on_iteration(d, &result);
                    completed = Some((d, result));
                }
                None => break,
            }
            d += 1;
        }
        completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::basic_movegen::basic_movegen;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

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
        let limits = SearchLimits {
            depth: Some(4),
            deadline: None,
            stop: &stop,
        };
        assert!(search
            .search_with_limits(&Chess::default(), &limits, &mut |_, _| {})
            .is_none());
        assert!(stop.load(Ordering::Relaxed));
    }

    #[test]
    fn deadline_prevents_an_unbounded_search() {
        let search = AlphaBetaIterativeDeepeningSearch::new(SearchConfig {
            evaluation_function: zero,
            move_generator: basic_movegen,
        });
        let stop = AtomicBool::new(false);
        let limits = SearchLimits {
            depth: None,
            deadline: Some(Instant::now() - Duration::from_millis(1)),
            stop: &stop,
        };

        assert!(search
            .search_with_limits(&Chess::default(), &limits, &mut |_, _| {})
            .is_none());
    }

    #[test]
    fn reports_each_completed_iteration() {
        let search = AlphaBetaIterativeDeepeningSearch::new(SearchConfig {
            evaluation_function: zero,
            move_generator: basic_movegen,
        });
        let stop = AtomicBool::new(false);
        let limits = SearchLimits {
            depth: Some(3),
            deadline: None,
            stop: &stop,
        };
        let mut depths = Vec::new();

        search.search_with_limits(&Chess::default(), &limits, &mut |depth, _| {
            depths.push(depth);
        });

        assert_eq!(depths, vec![1, 2, 3]);
    }
}
