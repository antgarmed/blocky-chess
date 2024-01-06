use std::time::SystemTime;

use shakmaty::Chess;

use super::{alphabeta::AlphaBetaSearch, Search, SearchConfig, SearchResult};

pub struct AlphaBetaIterativeDeepeningSearch {
    pub config: SearchConfig,
    alpha_beta_search: AlphaBetaSearch,
}

impl AlphaBetaIterativeDeepeningSearch {
    pub fn new(config: SearchConfig) -> Self {
        Self {
            config: config.clone(),
            alpha_beta_search: AlphaBetaSearch::new(config),
        }
    }
}

impl Search for AlphaBetaIterativeDeepeningSearch {
    fn search(&self, initial_position: &Chess, depth: usize) -> SearchResult {
        let mut search_result = SearchResult {
            value: (self.config.evaluation_function)(initial_position),
            principal_variation: Vec::new(),
        };

        for d in 1..=depth {
            search_result = self.alpha_beta_search.search(initial_position, d);
        }

        return search_result;
    }
}
