use super::{Search, SearchConfig, SearchResult, Value};
use crate::utils::consts::MATE_VALUE;
use shakmaty::{Chess, Color, Outcome, Position};

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
            0,
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
        ply_from_root: usize,
    ) -> SearchResult {
        if depth == 0 || position.outcome().is_some() {
            let value = match position.outcome() {
                Some(Outcome::Decisive { winner }) if winner.is_white() => {
                    MATE_VALUE - ply_from_root as i64
                }
                Some(Outcome::Decisive { .. }) => ply_from_root as i64 - MATE_VALUE,
                Some(Outcome::Draw) => 0,
                _ => (self.config.evaluation_function)(position),
            };

            return SearchResult {
                value,
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
                let child_result = self.alpha_beta_search(
                    &child_node,
                    depth - 1,
                    alpha,
                    beta,
                    Color::Black,
                    ply_from_root + 1,
                );

                if child_result.value > best_search_result.value {
                    best_search_result = child_result;
                    best_search_result.principal_variation.insert(0, m);
                }

                alpha = alpha.max(best_search_result.value);

                if beta <= alpha {
                    break;
                }
            }

            best_search_result
        } else {
            let mut best_search_result = SearchResult {
                value: Value::MAX,
                principal_variation: Vec::new(),
            };

            for m in moves {
                let child_node = position.clone().play(&m).unwrap();
                let child_result = self.alpha_beta_search(
                    &child_node,
                    depth - 1,
                    alpha,
                    beta,
                    Color::White,
                    ply_from_root + 1,
                );

                if child_result.value < best_search_result.value {
                    best_search_result = child_result;
                    best_search_result.principal_variation.insert(0, m);
                }

                beta = beta.min(best_search_result.value);

                if beta <= alpha {
                    break;
                }
            }

            best_search_result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::basic_movegen::basic_movegen;
    use crate::utils::consts::MATE_VALUE;
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Outcome};

    fn zero_evaluation(position: &Chess) -> Value {
        match position.outcome() {
            Some(Outcome::Decisive { winner }) if winner.is_white() => MATE_VALUE,
            Some(Outcome::Decisive { .. }) => -MATE_VALUE,
            Some(Outcome::Draw) | None => 0,
        }
    }

    const BASIC_CONFIG: SearchConfig = SearchConfig {
        evaluation_function: zero_evaluation,
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

        assert!(!result.principal_variation.is_empty());
    }

    #[test]
    fn test_search_reports_mate_in_1_found_before_horizon() {
        let fen: Fen = "7k/5Q2/6K1/8/8/8/8/8 w - - 0 1".parse().unwrap();
        let position: Chess = fen.into_position(CastlingMode::Standard).unwrap();

        let result = AlphaBetaSearch {
            config: BASIC_CONFIG,
        }
        .search(&position, 3);

        assert_eq!(result.value, MATE_VALUE - 1);
        assert_eq!(result.get_mate_in(), Some(1));
        assert_eq!(
            result.principal_variation[0]
                .to_uci(CastlingMode::Standard)
                .to_string(),
            "f7g7"
        );
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

        assert_eq!(result.get_mate_in(), Some(2));
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

        assert_eq!(result.get_mate_in(), Some(2));
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

    #[test]
    fn test_search_solves_mate_in_3_white_to_play_when_depth_is_6() {
        let fen: Fen = "2b2k1r/4rppp/p3p3/1pp1q1N1/8/7P/PPP3P1/3R1RK1 w - - 0 1"
            .parse()
            .unwrap();
        let position: Chess = fen.into_position(CastlingMode::Standard).unwrap();
        let depth = 6;

        let result = AlphaBetaSearch {
            config: BASIC_CONFIG,
        }
        .search(&position, depth);

        assert_eq!(result.get_mate_in(), Some(3));
    }

    #[test]
    fn test_search_solves_mate_in_3_black_to_play_when_depth_is_6() {
        let fen: Fen = "6k1/p1p3pp/4P3/3Q4/6PK/1P3r1P/P1P5/7r b - - 0 1"
            .parse()
            .unwrap();
        let position: Chess = fen.into_position(CastlingMode::Standard).unwrap();
        let depth = 6;

        let result = AlphaBetaSearch {
            config: BASIC_CONFIG,
        }
        .search(&position, depth);

        assert_eq!(result.get_mate_in(), Some(3));
    }
}
