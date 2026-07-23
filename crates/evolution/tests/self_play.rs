use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use blocky_chess::{EvaluationConfig, Search, SearchLimits, SearchResult};
use blocky_evolution::self_play::{
    DrawReason, GameError, GameOutcome, MoveSelectionError, MoveSelector, SearchMoveSelector,
    SearchMoveSelectorError, SelfPlayGame,
};
use shakmaty::{fen::Fen, uci::UciMove, CastlingMode, Chess, Color, Move, Position, Role, Square};

fn position(fen: &str) -> Chess {
    fen.parse::<Fen>()
        .expect("valid test FEN")
        .into_position(CastlingMode::Standard)
        .expect("legal test position")
}

#[derive(Default)]
struct ScriptedSelector {
    moves: VecDeque<&'static str>,
    calls: usize,
}

impl ScriptedSelector {
    fn new(moves: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            moves: moves.into_iter().collect(),
            calls: 0,
        }
    }
}

impl MoveSelector for ScriptedSelector {
    fn select_move(&mut self, position: &Chess) -> Result<Option<Move>, MoveSelectionError> {
        self.calls += 1;
        let Some(uci) = self.moves.pop_front() else {
            return Ok(None);
        };
        let uci = uci.parse::<UciMove>().expect("valid test UCI");
        Ok(Some(
            uci.to_move(position).expect("legal scripted test move"),
        ))
    }
}

#[test]
fn standard_game_starts_with_white_and_alternates_selectors() {
    let white = ScriptedSelector::new(["e2e4"]);
    let black = ScriptedSelector::new(["e7e5"]);

    let result = SelfPlayGame::standard(white, black, 2)
        .play()
        .expect("script is legal");

    assert_eq!(result.outcome, GameOutcome::Draw(DrawReason::MaxPlies));
    assert_eq!(result.moves.len(), 2);
    assert_eq!(result.position_history.len(), 3);
    assert_eq!(result.final_position.turn(), Color::White);
}

#[test]
fn known_mating_sequence_awards_victory_to_black() {
    let white = ScriptedSelector::new(["f2f3", "g2g4"]);
    let black = ScriptedSelector::new(["e7e5", "d8h4"]);

    let result = SelfPlayGame::standard(white, black, 4)
        .play()
        .expect("script is legal");

    assert_eq!(result.outcome, GameOutcome::BlackWin);
    assert_eq!(result.moves.len(), 4);
    assert!(result.final_position.is_checkmate());
}

#[test]
fn stalemate_is_drawn_without_invoking_a_selector() {
    struct CountingSelector(Arc<Mutex<usize>>);

    impl MoveSelector for CountingSelector {
        fn select_move(&mut self, _: &Chess) -> Result<Option<Move>, MoveSelectionError> {
            *self.0.lock().expect("test mutex") += 1;
            Ok(None)
        }
    }

    let calls = Arc::new(Mutex::new(0));
    let white = CountingSelector(Arc::clone(&calls));
    let black = CountingSelector(Arc::clone(&calls));
    let initial = position("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1");
    let result = SelfPlayGame::from_position(initial, white, black, 10)
        .play()
        .expect("terminal positions do not select");

    assert_eq!(result.outcome, GameOutcome::Draw(DrawReason::Stalemate));
    assert_eq!(*calls.lock().expect("test mutex"), 0);
}

#[test]
fn insufficient_material_is_drawn() {
    let initial = position("7k/8/8/8/8/8/8/K7 w - - 0 1");
    let result = SelfPlayGame::from_position(
        initial,
        ScriptedSelector::default(),
        ScriptedSelector::default(),
        10,
    )
    .play()
    .expect("terminal positions do not select");

    assert_eq!(
        result.outcome,
        GameOutcome::Draw(DrawReason::InsufficientMaterial)
    );
}

#[test]
fn third_occurrence_of_a_position_is_drawn() {
    let white = ScriptedSelector::new(["g1f3", "f3g1", "g1f3", "f3g1"]);
    let black = ScriptedSelector::new(["g8f6", "f6g8", "g8f6", "f6g8"]);

    let result = SelfPlayGame::standard(white, black, 20)
        .play()
        .expect("cycle is legal");

    assert_eq!(
        result.outcome,
        GameOutcome::Draw(DrawReason::ThreefoldRepetition)
    );
    assert_eq!(result.moves.len(), 8);
}

#[test]
fn hundred_halfmoves_is_drawn_by_the_fifty_move_rule() {
    let initial = position("7k/8/8/8/8/8/R7/K7 w - - 100 51");
    let result = SelfPlayGame::from_position(
        initial,
        ScriptedSelector::default(),
        ScriptedSelector::default(),
        10,
    )
    .play()
    .expect("rule is checked before selecting");

    assert_eq!(result.outcome, GameOutcome::Draw(DrawReason::FiftyMoveRule));
}

#[test]
fn hundredth_reversible_halfmove_triggers_the_fifty_move_rule() {
    let initial = position("7k/8/8/8/8/8/R7/K7 w - - 99 50");
    let result = SelfPlayGame::from_position(
        initial,
        ScriptedSelector::new(["a2a3"]),
        ScriptedSelector::default(),
        2,
    )
    .play()
    .expect("rook move is legal");

    assert_eq!(result.outcome, GameOutcome::Draw(DrawReason::FiftyMoveRule));
    assert_eq!(result.moves.len(), 1);
    assert_eq!(result.final_position.halfmoves(), 100);
}

#[test]
fn pawn_move_resets_the_fifty_move_counter() {
    let initial = position("7k/8/8/8/8/8/P7/K7 w - - 99 50");
    let result = SelfPlayGame::from_position(
        initial,
        ScriptedSelector::new(["a2a3"]),
        ScriptedSelector::default(),
        1,
    )
    .play()
    .expect("pawn move is legal");

    assert_eq!(result.outcome, GameOutcome::Draw(DrawReason::MaxPlies));
    assert_eq!(result.final_position.halfmoves(), 0);
}

#[test]
fn terminal_board_result_takes_precedence_over_draw_adjudication() {
    let initial = position("7k/6Q1/6K1/8/8/8/8/8 b - - 100 51");
    let result = SelfPlayGame::from_position(
        initial,
        ScriptedSelector::default(),
        ScriptedSelector::default(),
        0,
    )
    .play()
    .expect("terminal positions do not select");

    assert_eq!(result.outcome, GameOutcome::WhiteWin);
}

#[test]
fn maximum_plies_has_a_distinct_draw_reason() {
    let result = SelfPlayGame::standard(
        ScriptedSelector::new(["e2e4"]),
        ScriptedSelector::default(),
        1,
    )
    .play()
    .expect("first move is legal");

    assert_eq!(result.outcome, GameOutcome::Draw(DrawReason::MaxPlies));
    assert_eq!(result.moves.len(), 1);
}

struct IllegalSelector;

impl MoveSelector for IllegalSelector {
    fn select_move(&mut self, _: &Chess) -> Result<Option<Move>, MoveSelectionError> {
        Ok(Some(Move::Normal {
            role: Role::Knight,
            from: Square::G8,
            capture: None,
            to: Square::F6,
            promotion: None,
        }))
    }
}

#[test]
fn illegal_selector_move_is_an_error() {
    let error = SelfPlayGame::standard(IllegalSelector, ScriptedSelector::default(), 1)
        .play()
        .expect_err("black's knight cannot move for white");

    assert!(matches!(
        error,
        GameError::IllegalMove {
            color: Color::White,
            ..
        }
    ));
}

#[test]
fn missing_move_in_a_non_terminal_position_is_an_error() {
    let error = SelfPlayGame::standard(ScriptedSelector::default(), ScriptedSelector::default(), 1)
        .play()
        .expect_err("the initial position has legal moves");

    assert_eq!(
        error,
        GameError::NoMoveInNonTerminal {
            color: Color::White
        }
    );
}

struct RecordingSearch {
    depths: Arc<Mutex<Vec<Option<usize>>>>,
    configs: Arc<Mutex<Vec<EvaluationConfig>>>,
}

impl Search for RecordingSearch {
    fn set_evaluation_config(&self, config: EvaluationConfig) {
        self.configs.lock().expect("test mutex").push(config);
    }

    fn search_with_limits(
        &self,
        initial_position: &Chess,
        limits: &SearchLimits<'_>,
        _: &mut dyn FnMut(usize, &SearchResult),
    ) -> Option<(usize, SearchResult)> {
        self.depths.lock().expect("test mutex").push(limits.depth);
        Some((
            limits.depth.unwrap_or_default(),
            SearchResult {
                value: 0,
                principal_variation: initial_position
                    .legal_moves()
                    .first()
                    .copied()
                    .into_iter()
                    .collect(),
            },
        ))
    }
}

#[test]
fn search_adapters_keep_independent_configs_and_respect_depth() {
    let white_depths = Arc::new(Mutex::new(Vec::new()));
    let black_depths = Arc::new(Mutex::new(Vec::new()));
    let white_configs = Arc::new(Mutex::new(Vec::new()));
    let black_configs = Arc::new(Mutex::new(Vec::new()));
    let white_config = EvaluationConfig {
        pawn_value: 111,
        ..EvaluationConfig::default()
    };
    let black_config = EvaluationConfig {
        pawn_value: 222,
        ..EvaluationConfig::default()
    };
    let mut white = SearchMoveSelector::new(
        Box::new(RecordingSearch {
            depths: Arc::clone(&white_depths),
            configs: Arc::clone(&white_configs),
        }),
        white_config,
        3,
    )
    .expect("positive depth");
    let mut black = SearchMoveSelector::new(
        Box::new(RecordingSearch {
            depths: Arc::clone(&black_depths),
            configs: Arc::clone(&black_configs),
        }),
        black_config,
        5,
    )
    .expect("positive depth");

    white
        .select_move(&Chess::default())
        .expect("recording search completes");
    black
        .select_move(&Chess::default())
        .expect("recording search completes");

    assert_eq!(white.evaluation_config(), white_config);
    assert_eq!(black.evaluation_config(), black_config);
    assert_eq!(*white_depths.lock().expect("test mutex"), [Some(3)]);
    assert_eq!(*black_depths.lock().expect("test mutex"), [Some(5)]);
    assert_eq!(*white_configs.lock().expect("test mutex"), [white_config]);
    assert_eq!(*black_configs.lock().expect("test mutex"), [black_config]);
}

#[test]
fn production_alpha_beta_adapter_selects_a_legal_move() {
    let position = Chess::default();
    let mut selector =
        SearchMoveSelector::alpha_beta(EvaluationConfig::default(), 1).expect("positive depth");

    let selected = selector
        .select_move(&position)
        .expect("depth-one search completes")
        .expect("the initial position has a best move");

    assert!(position.is_legal(selected));
    assert_eq!(selector.depth(), 1);
}

#[test]
fn search_adapter_rejects_zero_depth() {
    let result = SearchMoveSelector::new(Box::new(CancelledSearch), EvaluationConfig::default(), 0);

    assert!(matches!(result, Err(SearchMoveSelectorError::ZeroDepth)));
}

#[test]
fn result_contains_moves_history_and_expected_final_position() {
    let expected = Chess::default()
        .play(
            "e2e4"
                .parse::<UciMove>()
                .expect("valid UCI")
                .to_move(&Chess::default())
                .expect("legal move"),
        )
        .expect("legal move");
    let result = SelfPlayGame::standard(
        ScriptedSelector::new(["e2e4"]),
        ScriptedSelector::default(),
        1,
    )
    .play()
    .expect("script is legal");

    assert_eq!(result.moves.len(), 1);
    assert_eq!(result.position_history.first(), Some(&Chess::default()));
    assert_eq!(result.position_history.last(), Some(&expected));
    assert_eq!(result.final_position, expected);
}

struct CancelledSearch;

impl Search for CancelledSearch {
    fn set_evaluation_config(&self, _: EvaluationConfig) {}

    fn search_with_limits(
        &self,
        _: &Chess,
        _: &SearchLimits<'_>,
        _: &mut dyn FnMut(usize, &SearchResult),
    ) -> Option<(usize, SearchResult)> {
        None
    }
}

#[test]
fn cancelled_or_missing_search_result_is_explicit() {
    let selector =
        SearchMoveSelector::new(Box::new(CancelledSearch), EvaluationConfig::default(), 4)
            .expect("positive depth");
    let error = SelfPlayGame::standard(selector, ScriptedSelector::default(), 1)
        .play()
        .expect_err("search did not complete");

    assert_eq!(
        error,
        GameError::SelectionFailed {
            color: Color::White,
            source: MoveSelectionError::SearchDidNotComplete,
        }
    );
}
