use crate::search::Search;
use shakmaty::{fen::Fen, uci::UciMove, CastlingMode, Chess, Color, Position};
use std::sync::Arc;

const ENGINE_NAME: &str = "Blocky";
const ENGINE_VERSION: &str = "0.1.0";
const ENGINE_AUTHOR: &str = "antgarmed";

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

    pub fn turn(&self) -> Color {
        self.position.turn()
    }

    pub fn set_default_position(&mut self) {
        self.position = Chess::default();
    }

    pub fn set_position_from_fen(&mut self, fen: &str) {
        let fen: Fen = fen.parse().unwrap();
        self.position = fen.into_position(CastlingMode::Standard).unwrap();
    }

    pub fn make_uci_move(&mut self, uci_move: &str) {
        let uci: UciMove = uci_move.parse().unwrap();
        let m = uci.to_move(&self.position).unwrap();
        self.position.play_unchecked(&m);
    }

    pub fn search_snapshot(&self) -> (Chess, Arc<dyn Search>, Color) {
        (
            self.position.clone(),
            Arc::clone(&self.search_algorithm),
            self.turn(),
        )
    }
}
