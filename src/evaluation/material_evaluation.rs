use crate::search::Value;
use shakmaty::{Chess, Position};

use super::{
    evaluate_outcome, get_number_of_black_bishops, get_number_of_black_knights,
    get_number_of_black_pawns, get_number_of_black_queens, get_number_of_black_rooks,
    get_number_of_white_bishops, get_number_of_white_knights, get_number_of_white_pawns,
    get_number_of_white_queens, get_number_of_white_rooks,
};

const PAWN_VALUE: Value = 100;
const KNIGHT_VALUE: Value = 300;
const BISHOP_VALUE: Value = 300;
const ROOK_VALUE: Value = 500;
const QUEEN_VALUE: Value = 900;

pub fn material_evaluation(position: &Chess) -> Value {
    if let Some(outcome) = position.outcome() {
        return evaluate_outcome(&outcome);
    }

    return QUEEN_VALUE
        * (get_number_of_white_queens(position) as i64
            - get_number_of_black_queens(position) as i64)
        + ROOK_VALUE
            * (get_number_of_white_rooks(position) as i64
                - get_number_of_black_rooks(position) as i64)
        + BISHOP_VALUE
            * (get_number_of_white_bishops(position) as i64
                - get_number_of_black_bishops(position) as i64)
        + KNIGHT_VALUE
            * (get_number_of_white_knights(position) as i64
                - get_number_of_black_knights(position) as i64)
        + PAWN_VALUE
            * (get_number_of_white_pawns(position) as i64
                - get_number_of_black_pawns(position) as i64);
}
