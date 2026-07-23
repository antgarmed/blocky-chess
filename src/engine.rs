use crate::evaluation::material_mobility_evaluation::MaterialMobilityConfig;
use crate::search::Search;
use shakmaty::{fen::Fen, uci::UciMove, CastlingMode, Chess, Color, Position};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

const ENGINE_NAME: &str = "Blocky";
const ENGINE_VERSION: &str = "0.1.0";
const ENGINE_AUTHOR: &str = "antgarmed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineInputError {
    InvalidFen { fen: String, reason: String },
    InvalidPosition { fen: String, reason: String },
    InvalidUciMove { uci_move: String, reason: String },
    IllegalMove { uci_move: String, reason: String },
}

impl fmt::Display for EngineInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFen { fen, reason } => {
                write!(formatter, "invalid FEN '{fen}': {reason}")
            }
            Self::InvalidPosition { fen, reason } => {
                write!(formatter, "invalid position from FEN '{fen}': {reason}")
            }
            Self::InvalidUciMove { uci_move, reason } => {
                write!(formatter, "invalid UCI move '{uci_move}': {reason}")
            }
            Self::IllegalMove { uci_move, reason } => {
                write!(formatter, "illegal UCI move '{uci_move}': {reason}")
            }
        }
    }
}

impl Error for EngineInputError {}

pub struct Engine {
    position: Chess,
    search_algorithm: Arc<dyn Search>,
}

impl Engine {
    pub fn new(search: Box<dyn Search>) -> Self {
        Self {
            position: Chess::default(),
            search_algorithm: Arc::from(search),
        }
    }

    pub fn get_full_name(&self) -> String {
        format!("{} {}", ENGINE_NAME, ENGINE_VERSION)
    }

    pub fn get_author(&self) -> String {
        ENGINE_AUTHOR.to_string()
    }

    pub fn set_evaluation_config(&self, config: MaterialMobilityConfig) {
        self.search_algorithm.set_evaluation_config(config);
    }

    pub fn turn(&self) -> Color {
        self.position.turn()
    }

    pub fn set_uci_position<I, S>(
        &mut self,
        fen: Option<&str>,
        moves: I,
    ) -> Result<(), EngineInputError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut position = match fen {
            Some(fen) => {
                let parsed: Fen = fen.parse().map_err(|error: shakmaty::fen::ParseFenError| {
                    EngineInputError::InvalidFen {
                        fen: fen.to_owned(),
                        reason: error.to_string(),
                    }
                })?;
                parsed
                    .into_position(CastlingMode::Standard)
                    .map_err(|error| EngineInputError::InvalidPosition {
                        fen: fen.to_owned(),
                        reason: error.to_string(),
                    })?
            }
            None => Chess::default(),
        };

        for uci_move in moves {
            let uci_move = uci_move.as_ref();
            let parsed: UciMove =
                uci_move
                    .parse()
                    .map_err(|error: shakmaty::uci::ParseUciMoveError| {
                        EngineInputError::InvalidUciMove {
                            uci_move: uci_move.to_owned(),
                            reason: error.to_string(),
                        }
                    })?;
            let chess_move =
                parsed
                    .to_move(&position)
                    .map_err(|error| EngineInputError::IllegalMove {
                        uci_move: uci_move.to_owned(),
                        reason: error.to_string(),
                    })?;
            position.play_unchecked(chess_move);
        }

        self.position = position;
        Ok(())
    }

    pub fn search_snapshot(&self) -> (Chess, Arc<dyn Search>, Color) {
        (
            self.position.clone(),
            Arc::clone(&self.search_algorithm),
            self.turn(),
        )
    }
}
