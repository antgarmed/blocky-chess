use shakmaty::{Chess, Color, Position, Role};

use crate::search::Value;

use super::{
    evaluate_outcome, get_legal_moves_for_color, material_evaluation::material_evaluation,
};

const MOBILITY_WEIGHT: Value = 10;
const MOBILITY_WEIGHT_SCALE: Value = 100;
const PAWN_MOBILITY_WEIGHT_HUNDREDTHS: Value = 5;
const KNIGHT_MOBILITY_WEIGHT_HUNDREDTHS: Value = 30;
const BISHOP_MOBILITY_WEIGHT_HUNDREDTHS: Value = 30;
const ROOK_MOBILITY_WEIGHT_HUNDREDTHS: Value = 20;
const QUEEN_MOBILITY_WEIGHT_HUNDREDTHS: Value = 10;
const KING_MOBILITY_WEIGHT_HUNDREDTHS: Value = 5;

pub fn material_mobility_evaluation(position: &Chess) -> Value {
    if let Some(outcome) = position.outcome() {
        return evaluate_outcome(&outcome);
    }

    let material_value = material_evaluation(position);
    let mobility_value = get_white_mobility(position) - get_black_mobility(position);

    material_value + MOBILITY_WEIGHT * mobility_value / MOBILITY_WEIGHT_SCALE
}

const fn mobility_weight(role: Role) -> Value {
    match role {
        Role::Pawn => PAWN_MOBILITY_WEIGHT_HUNDREDTHS,
        Role::Knight => KNIGHT_MOBILITY_WEIGHT_HUNDREDTHS,
        Role::Bishop => BISHOP_MOBILITY_WEIGHT_HUNDREDTHS,
        Role::Rook => ROOK_MOBILITY_WEIGHT_HUNDREDTHS,
        Role::Queen => QUEEN_MOBILITY_WEIGHT_HUNDREDTHS,
        Role::King => KING_MOBILITY_WEIGHT_HUNDREDTHS,
    }
}

fn get_weighted_number_of_moves_for_color(position: &Chess, color: Color) -> Value {
    get_legal_moves_for_color(position, color)
        .map(|moves| {
            moves
                .iter()
                .map(|chess_move| mobility_weight(chess_move.role()))
                .sum()
        })
        .unwrap_or(0)
}

fn get_weighted_number_of_moves_for_white(position: &Chess) -> Value {
    get_weighted_number_of_moves_for_color(position, Color::White)
}

fn get_weighted_number_of_moves_for_black(position: &Chess) -> Value {
    get_weighted_number_of_moves_for_color(position, Color::Black)
}

fn get_white_mobility(position: &Chess) -> Value {
    get_weighted_number_of_moves_for_white(position)
}

fn get_black_mobility(position: &Chess) -> Value {
    get_weighted_number_of_moves_for_black(position)
}
