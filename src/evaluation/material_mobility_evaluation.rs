use shakmaty::{Chess, Color, Position};

use crate::search::Value;

use super::{evaluate_outcome, material_evaluation::material_evaluation};

pub fn material_mobility_evaluation(position: &Chess) -> Value {
    if let Some(outcome) = position.outcome() {
        return evaluate_outcome(&outcome);
    }

    let material_value = material_evaluation(position);
    let mobility_value = get_white_mobility(position) - get_black_mobility(position);

    material_value + 1000 * mobility_value
}

fn get_white_mobility(position: &Chess) -> Value {
    if position.turn() == Color::Black {
        return position.clone().legal_moves().len() as i64;
    }

    position.legal_moves().len() as i64
}

fn get_black_mobility(position: &Chess) -> Value {
    if position.turn() == Color::White {
        return position.clone().legal_moves().len() as i64;
    }

    position.legal_moves().len() as i64
}
