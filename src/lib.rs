//! Reusable chess engine library and UCI adapter.

pub mod engine;
pub mod evaluation;
pub mod movegen;
pub mod search;
pub mod uci;
pub mod utils;

pub use engine::{Engine, EngineInputError};
pub use evaluation::EvaluationConfig;
pub use search::{Search, SearchConfig, SearchLimits, SearchResult, Value};
