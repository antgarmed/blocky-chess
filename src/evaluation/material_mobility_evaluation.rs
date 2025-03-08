use shakmaty::{Chess, Position};

use crate::search::Value;

use super::{
    evaluate_outcome, get_number_of_moves_for_black, get_number_of_moves_for_white,
    material_evaluation::material_evaluation,
};

pub fn material_mobility_evaluation(position: &Chess) -> Value {
    if let Some(outcome) = position.outcome() {
        return evaluate_outcome(&outcome);
    }

    let material_value = material_evaluation(position);
    let mobility_value = get_white_mobility(position) - get_black_mobility(position);

    mobility_value
}

fn get_white_mobility(position: &Chess) -> Value {
    get_number_of_moves_for_white(position) as i64
}

fn get_black_mobility(position: &Chess) -> Value {
    get_number_of_moves_for_black(position) as i64
}
