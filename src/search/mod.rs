use shakmaty::{Chess, Move};

pub type Value = i64;

pub struct SearchResult {
    pub value: Value,
    pub principal_variation: Vec<Move>,
}

pub struct SearchConfig {
    pub evaluation_function: fn(&Chess) -> Value,
    pub move_generator: fn(&Chess) -> Vec<Move>,
}

pub trait SearchFactory {
    fn build(&self, config: SearchConfig) -> Box<dyn Search>;
}

pub trait Search {
    fn search(&self, initial_position: &mut Chess, depth: usize) -> SearchResult;
}

pub mod alphabeta;
