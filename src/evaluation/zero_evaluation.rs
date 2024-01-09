use super::evaluate_outcome;
use crate::search::Value;
use shakmaty::{Chess, Position};

pub fn zero_evaluation(position: &Chess) -> Value {
    if let Some(outcome) = position.outcome() {
        return evaluate_outcome(&outcome);
    }

    0
}
