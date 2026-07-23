use crate::search::Value;
use shakmaty::{Chess, Position};

use super::{
    evaluate_outcome, get_number_of_black_bishops, get_number_of_black_knights,
    get_number_of_black_pawns, get_number_of_black_queens, get_number_of_black_rooks,
    get_number_of_white_bishops, get_number_of_white_knights, get_number_of_white_pawns,
    get_number_of_white_queens, get_number_of_white_rooks, EvaluationConfig,
};

pub fn material_evaluation(position: &Chess, config: &EvaluationConfig) -> Value {
    let outcome = position.outcome();
    if outcome.is_known() {
        return evaluate_outcome(&outcome);
    }

    config.queen_value
        * (get_number_of_white_queens(position) as i64
            - get_number_of_black_queens(position) as i64)
        + config.rook_value
            * (get_number_of_white_rooks(position) as i64
                - get_number_of_black_rooks(position) as i64)
        + config.bishop_value
            * (get_number_of_white_bishops(position) as i64
                - get_number_of_black_bishops(position) as i64)
        + config.knight_value
            * (get_number_of_white_knights(position) as i64
                - get_number_of_black_knights(position) as i64)
        + config.pawn_value
            * (get_number_of_white_pawns(position) as i64
                - get_number_of_black_pawns(position) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::{fen::Fen, CastlingMode};

    #[test]
    fn material_values_are_read_from_the_evaluation_config() {
        let position: Chess = Fen::from_ascii(b"4k3/8/8/8/8/8/P7/6K1 w - - 0 1")
            .unwrap()
            .into_position(CastlingMode::Standard)
            .unwrap();
        let config = EvaluationConfig {
            pawn_value: 250,
            ..EvaluationConfig::default()
        };

        assert_eq!(material_evaluation(&position, &config), 250);
    }
}
