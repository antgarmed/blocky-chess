use crate::{search::Value, utils::consts::MATE_VALUE};
use shakmaty::{Chess, Color, Outcome, Position};

pub fn zero_evaluation(position: &Chess) -> Value {
    if let Some(outcome) = position.outcome() {
        match outcome {
            Outcome::Decisive { winner } => {
                return match winner {
                    Color::White => MATE_VALUE,
                    Color::Black => -MATE_VALUE,
                };
            }
            Outcome::Draw => return 0,
        }
    }

    0
}
