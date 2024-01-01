use super::{Search, SearchConfig, SearchResult, Value};
use crate::utils::consts::MATE_VALUE;
use shakmaty::{Chess, Color, Position};

const INITIAL_ALPHA: Value = Value::MIN;
const INITIAL_BETA: Value = Value::MAX;

pub struct AlphaBetaSearch {
    pub config: SearchConfig,
}

impl AlphaBetaSearch {
    pub fn new(config: SearchConfig) -> Self {
        Self { config }
    }
}

impl Search for AlphaBetaSearch {
    fn search(&self, initial_position: &Chess, depth: usize) -> SearchResult {
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

        let moves = (self.config.move_generator)(position);

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
                    best_search_result = child_result;
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

                if child_result.value < best_search_result.value {
                    best_search_result = child_result;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::zero_evalution::zero_evalution;
    use crate::movegen::basic_movegen::basic_movegen;
    use shakmaty::fen::Fen;
    use shakmaty::CastlingMode;

    const BASIC_CONFIG: SearchConfig = SearchConfig {
        evaluation_function: zero_evalution,
        move_generator: basic_movegen,
    };

    #[test]
    fn test_search_returns_result_when_depth_is_1() {
        let position = Chess::default();
        let depth = 1;

        let result = AlphaBetaSearch {
            config: BASIC_CONFIG,
        }
        .search(&position, depth);

        assert!(result.principal_variation.len() >= 1);
    }

    #[test]
    fn test_search_solves_mate_in_2_white_to_play_when_depth_is_4() {
        let fen: Fen = "r4r1k/b1p3pp/p2P2p1/1p6/1P4R1/1B5Q/Pq3P2/R5K1 w - - 0 1"
            .parse()
            .unwrap();
        let position: Chess = fen.into_position(CastlingMode::Standard).unwrap();
        let depth = 4;

        let result = AlphaBetaSearch {
            config: BASIC_CONFIG,
        }
        .search(&position, depth);

        assert_eq!(
            result.principal_variation[0]
                .to_uci(CastlingMode::Standard)
                .to_string(),
            "h3h7"
        );
        assert_eq!(
            result.principal_variation[1]
                .to_uci(CastlingMode::Standard)
                .to_string(),
            "h8h7"
        );
        assert_eq!(
            result.principal_variation[2]
                .to_uci(CastlingMode::Standard)
                .to_string(),
            "g4h4"
        );
    }

    #[test]
    fn test_search_solves_mate_in_2_black_to_play_when_depth_is_4() {
        let fen: Fen = "1r2nk2/3n4/pB1P4/2P4p/3Q1P1q/4P1p1/5P2/RR4K1 b - - 0 1"
            .parse()
            .unwrap();
        let position: Chess = fen.into_position(CastlingMode::Standard).unwrap();
        let depth = 4;

        let result = AlphaBetaSearch {
            config: BASIC_CONFIG,
        }
        .search(&position, depth);

        assert_eq!(
            result.principal_variation[0]
                .to_uci(CastlingMode::Standard)
                .to_string(),
            "h4h2"
        );
        assert_eq!(
            result.principal_variation[1]
                .to_uci(CastlingMode::Standard)
                .to_string(),
            "g1f1"
        );
        assert_eq!(
            result.principal_variation[2]
                .to_uci(CastlingMode::Standard)
                .to_string(),
            "h2f2"
        );
    }
}
