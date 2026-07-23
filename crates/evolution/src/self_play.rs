//! Deterministic, in-process self-play arbitration.

use std::{
    collections::HashMap,
    error::Error,
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};

use blocky_chess::{
    evaluation::main_evaluation::main_evaluation, movegen::basic_movegen::basic_movegen,
    search::alphabeta::AlphaBetaSearch, EvaluationConfig, Search, SearchConfig, SearchLimits,
};
use shakmaty::{zobrist::Zobrist128, Chess, Color, EnPassantMode, Move, Position};

/// Selects one move without taking responsibility for game rules.
///
/// Keeping this boundary small makes the arbiter independent from the search
/// implementation and allows cheap, deterministic scripted tests.
pub trait MoveSelector {
    fn select_move(&mut self, position: &Chess) -> Result<Option<Move>, MoveSelectionError>;
}

/// A failure inside a move selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveSelectionError {
    /// The search was cancelled or otherwise returned no completed result.
    SearchDidNotComplete,
}

impl fmt::Display for MoveSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SearchDidNotComplete => {
                formatter.write_str("search did not produce a completed result")
            }
        }
    }
}

impl Error for MoveSelectionError {}

/// Adapts Blocky Chess's search API to [`MoveSelector`].
pub struct SearchMoveSelector {
    search: Box<dyn Search>,
    evaluation_config: EvaluationConfig,
    depth: usize,
    stop: AtomicBool,
}

/// Invalid construction of a [`SearchMoveSelector`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMoveSelectorError {
    ZeroDepth,
}

impl fmt::Display for SearchMoveSelectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDepth => formatter.write_str("search depth must be greater than zero"),
        }
    }
}

impl Error for SearchMoveSelectorError {}

impl SearchMoveSelector {
    /// Wraps a search implementation, assigning this selector its own config.
    pub fn new(
        search: Box<dyn Search>,
        evaluation_config: EvaluationConfig,
        depth: usize,
    ) -> Result<Self, SearchMoveSelectorError> {
        if depth == 0 {
            return Err(SearchMoveSelectorError::ZeroDepth);
        }

        search.set_evaluation_config(evaluation_config);
        Ok(Self {
            search,
            evaluation_config,
            depth,
            stop: AtomicBool::new(false),
        })
    }

    /// Builds a production selector backed by an independent alpha-beta search.
    pub fn alpha_beta(
        evaluation_config: EvaluationConfig,
        depth: usize,
    ) -> Result<Self, SearchMoveSelectorError> {
        let search_config = SearchConfig {
            evaluation_function: main_evaluation,
            move_generator: basic_movegen,
            evaluation_config: Arc::new(RwLock::new(evaluation_config)),
        };
        Self::new(
            Box::new(AlphaBetaSearch::new(search_config)),
            evaluation_config,
            depth,
        )
    }

    pub fn evaluation_config(&self) -> EvaluationConfig {
        self.evaluation_config
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Requests cancellation of this selector's current or next search.
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl MoveSelector for SearchMoveSelector {
    fn select_move(&mut self, position: &Chess) -> Result<Option<Move>, MoveSelectionError> {
        let result = self.search.search_with_limits(
            position,
            &SearchLimits {
                depth: Some(self.depth),
                deadline: None,
                stop: &self.stop,
            },
            &mut |_, _| {},
        );

        result
            .map(|(_, result)| result.principal_variation.first().copied())
            .ok_or(MoveSelectionError::SearchDidNotComplete)
    }
}

/// Why an otherwise undecided game was adjudicated as a draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawReason {
    Stalemate,
    InsufficientMaterial,
    ThreefoldRepetition,
    FiftyMoveRule,
    MaxPlies,
}

/// The result of a completed self-play game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameOutcome {
    WhiteWin,
    BlackWin,
    Draw(DrawReason),
}

/// A completed game, including enough state to inspect or reproduce it.
#[derive(Clone, Debug)]
pub struct GameRecord {
    pub outcome: GameOutcome,
    pub moves: Vec<Move>,
    /// Includes the injected initial position and every position after a move.
    pub position_history: Vec<Chess>,
    pub final_position: Chess,
}

/// A failure to arbitrate a game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameError {
    SelectionFailed {
        color: Color,
        source: MoveSelectionError,
    },
    NoMoveInNonTerminal {
        color: Color,
    },
    IllegalMove {
        color: Color,
        attempted: Move,
    },
}

impl fmt::Display for GameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectionFailed { color, source } => {
                write!(formatter, "{color:?} move selection failed: {source}")
            }
            Self::NoMoveInNonTerminal { color } => {
                write!(
                    formatter,
                    "{color:?} selector returned no move in a non-terminal position"
                )
            }
            Self::IllegalMove { color, attempted } => {
                write!(
                    formatter,
                    "{color:?} selector returned illegal move {attempted:?}"
                )
            }
        }
    }
}

impl Error for GameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SelectionFailed { source, .. } => Some(source),
            Self::NoMoveInNonTerminal { .. } | Self::IllegalMove { .. } => None,
        }
    }
}

/// Owns both players and arbitrates one deterministic game.
pub struct SelfPlayGame<White, Black> {
    white: White,
    black: Black,
    initial_position: Chess,
    max_plies: usize,
}

impl<White, Black> SelfPlayGame<White, Black>
where
    White: MoveSelector,
    Black: MoveSelector,
{
    pub fn standard(white: White, black: Black, max_plies: usize) -> Self {
        Self::from_position(Chess::default(), white, black, max_plies)
    }

    pub fn from_position(
        initial_position: Chess,
        white: White,
        black: Black,
        max_plies: usize,
    ) -> Self {
        Self {
            white,
            black,
            initial_position,
            max_plies,
        }
    }

    pub fn play(mut self) -> Result<GameRecord, GameError> {
        let mut position = self.initial_position;
        let mut moves = Vec::new();
        let mut position_history = vec![position.clone()];
        let mut repetitions = HashMap::new();
        repetitions.insert(position_key(&position), 1_u8);

        loop {
            if let Some(outcome) = board_outcome(&position) {
                return Ok(record(outcome, moves, position_history, position));
            }
            if position.halfmoves() >= 100 {
                return Ok(record(
                    GameOutcome::Draw(DrawReason::FiftyMoveRule),
                    moves,
                    position_history,
                    position,
                ));
            }
            if repetitions
                .get(&position_key(&position))
                .is_some_and(|occurrences| *occurrences >= 3)
            {
                return Ok(record(
                    GameOutcome::Draw(DrawReason::ThreefoldRepetition),
                    moves,
                    position_history,
                    position,
                ));
            }
            if moves.len() >= self.max_plies {
                return Ok(record(
                    GameOutcome::Draw(DrawReason::MaxPlies),
                    moves,
                    position_history,
                    position,
                ));
            }

            let color = position.turn();
            let selected = if color.is_white() {
                self.white.select_move(&position)
            } else {
                self.black.select_move(&position)
            }
            .map_err(|source| GameError::SelectionFailed { color, source })?;
            let selected = selected.ok_or(GameError::NoMoveInNonTerminal { color })?;

            if !position.is_legal(selected) {
                return Err(GameError::IllegalMove {
                    color,
                    attempted: selected,
                });
            }

            position = position
                .play(selected)
                .map_err(|error| GameError::IllegalMove {
                    color,
                    attempted: error.m,
                })?;
            moves.push(selected);
            position_history.push(position.clone());
            let occurrences = repetitions.entry(position_key(&position)).or_insert(0);
            *occurrences = occurrences.saturating_add(1);
        }
    }
}

fn board_outcome(position: &Chess) -> Option<GameOutcome> {
    if position.legal_moves().is_empty() {
        if position.is_check() {
            Some(if position.turn().is_white() {
                GameOutcome::BlackWin
            } else {
                GameOutcome::WhiteWin
            })
        } else {
            Some(GameOutcome::Draw(DrawReason::Stalemate))
        }
    } else if position.is_insufficient_material() {
        Some(GameOutcome::Draw(DrawReason::InsufficientMaterial))
    } else {
        None
    }
}

fn position_key(position: &Chess) -> u128 {
    position.zobrist_hash::<Zobrist128>(EnPassantMode::Legal).0
}

fn record(
    outcome: GameOutcome,
    moves: Vec<Move>,
    position_history: Vec<Chess>,
    final_position: Chess,
) -> GameRecord {
    GameRecord {
        outcome,
        moves,
        position_history,
        final_position,
    }
}
