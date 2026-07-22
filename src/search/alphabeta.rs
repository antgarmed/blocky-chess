use super::{Search, SearchConfig, SearchResult, Value};
use crate::utils::consts::MATE_VALUE;
use shakmaty::{Chess, Color, Outcome, Position};
use std::sync::atomic::{AtomicBool, Ordering};

const INITIAL_ALPHA: Value = Value::MIN;
const INITIAL_BETA: Value = Value::MAX;

#[derive(Clone, Copy)]
struct SearchState {
    alpha: Value,
    beta: Value,
    color_to_maximize: Color,
    ply_from_root: usize,
}

pub struct AlphaBetaSearch {
    pub config: SearchConfig,
}

impl AlphaBetaSearch {
    pub fn new(config: SearchConfig) -> Self {
        Self { config }
    }
}

impl Search for AlphaBetaSearch {
    fn search_with_stop(
        &self,
        initial_position: &Chess,
        depth: usize,
        stop: &AtomicBool,
    ) -> Option<(usize, SearchResult)> {
        self.alpha_beta_search_with_stop(
            initial_position,
            depth,
            SearchState {
                alpha: INITIAL_ALPHA,
                beta: INITIAL_BETA,
                color_to_maximize: initial_position.turn(),
                ply_from_root: 0,
            },
            stop,
        )
        .map(|result| (depth, result))
    }
}

impl AlphaBetaSearch {
    fn alpha_beta_search_with_stop(
        &self,
        position: &Chess,
        depth: usize,
        mut state: SearchState,
        stop: &AtomicBool,
    ) -> Option<SearchResult> {
        if stop.load(Ordering::Relaxed) {
            return None;
        }
        if depth == 0 || position.outcome().is_some() {
            let value = match position.outcome() {
                Some(Outcome::Decisive { winner }) if winner.is_white() => {
                    MATE_VALUE - state.ply_from_root as i64
                }
                Some(Outcome::Decisive { .. }) => state.ply_from_root as i64 - MATE_VALUE,
                Some(Outcome::Draw) => 0,
                _ => (self.config.evaluation_function)(position),
            };
            return Some(SearchResult {
                value,
                principal_variation: Vec::new(),
            });
        }

        let maximizing = state.color_to_maximize.is_white();
        let mut best = SearchResult {
            value: if maximizing { Value::MIN } else { Value::MAX },
            principal_variation: Vec::new(),
        };
        for m in (self.config.move_generator)(position) {
            if stop.load(Ordering::Relaxed) {
                return None;
            }
            let child = position.clone().play(&m).unwrap();
            let child_result = self.alpha_beta_search_with_stop(
                &child,
                depth - 1,
                SearchState {
                    alpha: state.alpha,
                    beta: state.beta,
                    color_to_maximize: !state.color_to_maximize,
                    ply_from_root: state.ply_from_root + 1,
                },
                stop,
            )?;
            if (maximizing && child_result.value > best.value)
                || (!maximizing && child_result.value < best.value)
            {
                best = child_result;
                best.principal_variation.insert(0, m);
            }
            if maximizing {
                state.alpha = state.alpha.max(best.value);
            } else {
                state.beta = state.beta.min(best.value);
            }
            if state.beta <= state.alpha {
                break;
            }
        }
        Some(best)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::basic_movegen::basic_movegen;
    use crate::utils::consts::MATE_VALUE;
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Outcome};
    use std::sync::atomic::AtomicBool;

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

    fn search(position: &Chess, depth: usize) -> SearchResult {
        AlphaBetaSearch {
            config: BASIC_CONFIG,
        }
        .search_with_stop(position, depth, &AtomicBool::new(false))
        .expect("search without cancellation must complete")
        .1
    }

    #[test]
    fn test_search_returns_result_when_depth_is_1() {
        let position = Chess::default();
        let depth = 1;

        let result = search(&position, depth);

        assert!(!result.principal_variation.is_empty());
    }

    #[test]
    fn test_search_reports_mate_in_1_found_before_horizon() {
        let fen: Fen = "7k/5Q2/6K1/8/8/8/8/8 w - - 0 1".parse().unwrap();
        let position: Chess = fen.into_position(CastlingMode::Standard).unwrap();

        let result = search(&position, 3);

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

        let result = search(&position, depth);

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

        let result = search(&position, depth);

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

        let result = search(&position, depth);

        assert_eq!(result.get_mate_in(), Some(3));
    }

    #[test]
    fn test_search_solves_mate_in_3_black_to_play_when_depth_is_6() {
        let fen: Fen = "6k1/p1p3pp/4P3/3Q4/6PK/1P3r1P/P1P5/7r b - - 0 1"
            .parse()
            .unwrap();
        let position: Chess = fen.into_position(CastlingMode::Standard).unwrap();
        let depth = 6;

        let result = search(&position, depth);

        assert_eq!(result.get_mate_in(), Some(3));
    }
}
