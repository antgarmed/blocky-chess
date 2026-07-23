use super::{Search, SearchConfig, SearchLimits, SearchResult, Value};
use crate::evaluation::material_mobility_evaluation::MaterialMobilityConfig;
use crate::utils::consts::MATE_VALUE;
use shakmaty::{Chess, Color, KnownOutcome, Outcome, Position};

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
    fn set_evaluation_config(&self, config: MaterialMobilityConfig) {
        *self.config.evaluation_config.write().unwrap() = config;
    }

    fn search_with_limits(
        &self,
        initial_position: &Chess,
        limits: &SearchLimits<'_>,
        _on_iteration: &mut dyn FnMut(usize, &SearchResult),
    ) -> Option<(usize, SearchResult)> {
        let depth = limits.depth.unwrap_or(usize::MAX);
        self.search_depth_with_limits(initial_position, depth, limits)
    }
}

impl AlphaBetaSearch {
    pub fn search_depth_with_limits(
        &self,
        initial_position: &Chess,
        depth: usize,
        limits: &SearchLimits<'_>,
    ) -> Option<(usize, SearchResult)> {
        self.alpha_beta_search_with_limits(
            initial_position,
            depth,
            SearchState {
                alpha: INITIAL_ALPHA,
                beta: INITIAL_BETA,
                color_to_maximize: initial_position.turn(),
                ply_from_root: 0,
            },
            limits,
        )
        .map(|result| (depth, result))
    }

    fn alpha_beta_search_with_limits(
        &self,
        position: &Chess,
        depth: usize,
        mut state: SearchState,
        limits: &SearchLimits<'_>,
    ) -> Option<SearchResult> {
        if limits.should_stop() {
            return None;
        }
        let outcome = position.outcome();
        if depth == 0 || outcome.is_known() {
            let value = match outcome {
                Outcome::Known(KnownOutcome::Decisive { winner }) if winner.is_white() => {
                    MATE_VALUE - state.ply_from_root as i64
                }
                Outcome::Known(KnownOutcome::Decisive { .. }) => {
                    state.ply_from_root as i64 - MATE_VALUE
                }
                Outcome::Known(KnownOutcome::Draw) => 0,
                _ => {
                    let config = self.config.evaluation_config.read().unwrap();
                    (self.config.evaluation_function)(position, &config)
                }
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
            if limits.should_stop() {
                return None;
            }
            let child = position.clone().play(m).unwrap();
            let child_result = self.alpha_beta_search_with_limits(
                &child,
                depth - 1,
                SearchState {
                    alpha: state.alpha,
                    beta: state.beta,
                    color_to_maximize: !state.color_to_maximize,
                    ply_from_root: state.ply_from_root + 1,
                },
                limits,
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
    use shakmaty::{CastlingMode, KnownOutcome, Outcome};
    use std::sync::atomic::AtomicBool;

    fn zero_evaluation(position: &Chess, _: &MaterialMobilityConfig) -> Value {
        match position.outcome() {
            Outcome::Known(KnownOutcome::Decisive { winner }) if winner.is_white() => MATE_VALUE,
            Outcome::Known(KnownOutcome::Decisive { .. }) => -MATE_VALUE,
            Outcome::Known(KnownOutcome::Draw) | Outcome::Unknown => 0,
        }
    }

    fn basic_config() -> SearchConfig {
        SearchConfig {
            evaluation_function: zero_evaluation,
            move_generator: basic_movegen,
            evaluation_config: std::sync::Arc::new(std::sync::RwLock::new(
                MaterialMobilityConfig::default(),
            )),
        }
    }

    fn search(position: &Chess, depth: usize) -> SearchResult {
        AlphaBetaSearch {
            config: basic_config(),
        }
        .search_with_limits(
            position,
            &SearchLimits {
                depth: Some(depth),
                deadline: None,
                stop: &AtomicBool::new(false),
            },
            &mut |_, _| {},
        )
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
