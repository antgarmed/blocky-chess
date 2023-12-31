use shakmaty::{Chess, Color, Position};

use super::{Search, SearchConfig, SearchResult, Value};

const INITIAL_ALPHA: Value = Value::MIN;
const INITIAL_BETA: Value = Value::MAX;
const MATE_VALUE: Value = 1000000;

pub struct AlphaBetaSearch {
    pub config: SearchConfig,
}

impl Search for AlphaBetaSearch {
    fn search(&self, initial_position: &mut Chess, depth: usize) -> SearchResult {
        self.alpha_beta_search(
            initial_position,
            depth,
            INITIAL_ALPHA,
            INITIAL_BETA,
            initial_position.turn(),
        )
    }
}

impl AlphaBetaSearch {
    fn alpha_beta_search(
        &self,
        position: &Chess,
        depth: usize,
        mut alpha: Value,
        mut beta: Value,
        color_to_maximize: Color,
    ) -> SearchResult {
        if depth == 0 || position.outcome().is_some() {
            return SearchResult {
                value: (self.config.evaluation_function)(position),
                principal_variation: Vec::new(),
            };
        }

        let moves = position.legal_moves();

        if color_to_maximize.is_white() {
            let mut best_search_result = SearchResult {
                value: Value::MIN,
                principal_variation: Vec::new(),
            };

            for m in moves {
                let child_node = position.clone().play(&m).unwrap();
                let mut child_result =
                    self.alpha_beta_search(&child_node, depth - 1, alpha, beta, Color::Black);

                if child_result.value == MATE_VALUE {
                    child_result.value -= child_result.principal_variation.len() as i64
                } else if child_result.value == -MATE_VALUE {
                    child_result.value += child_result.principal_variation.len() as i64
                }

                if child_result.value > best_search_result.value {
                    best_search_result.value = child_result.value;
                    best_search_result.principal_variation.insert(0, m);
                }

                alpha = alpha.max(best_search_result.value);

                if beta <= alpha {
                    break;
                }
            }

            return best_search_result;
        } else {
            let mut best_search_result = SearchResult {
                value: Value::MAX,
                principal_variation: Vec::new(),
            };

            for m in moves {
                let child_node = position.clone().play(&m).unwrap();
                let mut child_result =
                    self.alpha_beta_search(&child_node, depth - 1, alpha, beta, Color::White);

                if child_result.value == MATE_VALUE {
                    child_result.value -= child_result.principal_variation.len() as i64
                } else if child_result.value == -MATE_VALUE {
                    child_result.value += child_result.principal_variation.len() as i64
                }

                if child_result.value > best_search_result.value {
                    best_search_result.value = child_result.value;
                    best_search_result.principal_variation.insert(0, m);
                }

                beta = beta.min(best_search_result.value);

                if beta <= alpha {
                    break;
                }
            }

            return best_search_result;
        }
    }
}
