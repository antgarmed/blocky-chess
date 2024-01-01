use crate::search::{Search, SearchResult};
use shakmaty::{fen::Fen, uci::Uci, CastlingMode, Chess, Position};

const ENGINE_NAME: &str = "Blocky";
const ENGINE_VERSION: &str = "0.1.0";
const ENGINE_AUTHOR: &str = "antgarmed";

pub struct Engine {
    position: Chess,
    search_algorithm: Box<dyn Search>,
}

impl Engine {
    pub fn new(search: Box<dyn Search>) -> Self {
        Self {
            position: Chess::default(),
            search_algorithm: search,
        }
    }

    pub fn get_full_name(&self) -> String {
        format!("{} {}", ENGINE_NAME, ENGINE_VERSION)
    }

    pub fn get_author(&self) -> String {
        ENGINE_AUTHOR.to_string()
    }

    pub fn set_default_position(&mut self) {
        self.position = Chess::default();
    }

    pub fn set_position_from_fen(&mut self, fen: &str) {
        let fen: Fen = fen.parse().unwrap();
        self.position = fen.into_position(CastlingMode::Standard).unwrap();
    }

    pub fn make_uci_move(&mut self, uci_move: &str) {
        let uci: Uci = uci_move.parse().unwrap();
        let m = uci.to_move(&self.position).unwrap();
        self.position.play_unchecked(&m);
    }

    pub fn go(&self, depth: usize) -> SearchResult {
        self.search_algorithm.search(&self.position, depth)
    }
}
