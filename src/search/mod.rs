use shakmaty::{Chess, Move, MoveList};

pub type Value = i64;

pub struct SearchResult {
    pub value: Value,
    pub principal_variation: Vec<Move>,
}

pub struct SearchConfig {
    pub evaluation_function: fn(&Chess) -> Value,
    pub move_generator: fn(&Chess) -> MoveList,
}

pub trait SearchFactory {
    fn build(&self, config: SearchConfig) -> Box<dyn Search>;
}

pub trait Search {
    fn search(&self, initial_position: &Chess, depth: usize) -> SearchResult;
}

pub mod alphabeta;
