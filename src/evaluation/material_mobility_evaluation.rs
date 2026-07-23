use shakmaty::{Chess, Color, Role};

use crate::search::Value;

use super::{get_legal_moves_for_color, EvaluationConfig};

const MOBILITY_WEIGHT_SCALE: Value = 100;

pub fn mobility_evaluation_with_config(position: &Chess, config: &EvaluationConfig) -> Value {
    let mobility_value =
        get_white_mobility(position, config) - get_black_mobility(position, config);

    config.mobility_weight * mobility_value / MOBILITY_WEIGHT_SCALE
}

const fn mobility_weight(role: Role, config: &EvaluationConfig) -> Value {
    match role {
        Role::Pawn => config.pawn_mobility_weight,
        Role::Knight => config.knight_mobility_weight,
        Role::Bishop => config.bishop_mobility_weight,
        Role::Rook => config.rook_mobility_weight,
        Role::Queen => config.queen_mobility_weight,
        Role::King => config.king_mobility_weight,
    }
}

fn get_weighted_number_of_moves_for_color(
    position: &Chess,
    color: Color,
    config: &EvaluationConfig,
) -> Value {
    get_legal_moves_for_color(position, color)
        .map(|moves| {
            moves
                .iter()
                .map(|chess_move| mobility_weight(chess_move.role(), config))
                .sum()
        })
        .unwrap_or(0)
}

fn get_weighted_number_of_moves_for_white(position: &Chess, config: &EvaluationConfig) -> Value {
    get_weighted_number_of_moves_for_color(position, Color::White, config)
}

fn get_weighted_number_of_moves_for_black(position: &Chess, config: &EvaluationConfig) -> Value {
    get_weighted_number_of_moves_for_color(position, Color::Black, config)
}

fn get_white_mobility(position: &Chess, config: &EvaluationConfig) -> Value {
    get_weighted_number_of_moves_for_white(position, config)
}

fn get_black_mobility(position: &Chess, config: &EvaluationConfig) -> Value {
    get_weighted_number_of_moves_for_black(position, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::{fen::Fen, CastlingMode};

    #[test]
    fn mobility_evaluation_is_stable_for_the_default_config() {
        let position: Chess = Fen::from_ascii(b"4k3/8/8/8/8/8/4P3/4K3 w - - 0 1")
            .unwrap()
            .into_position(CastlingMode::Standard)
            .unwrap();

        assert_eq!(
            mobility_evaluation_with_config(&position, &EvaluationConfig::default()),
            mobility_evaluation_with_config(&position, &EvaluationConfig::default())
        );
    }

    #[test]
    fn piece_mobility_weights_are_read_from_the_configuration() {
        let position: Chess = Fen::from_ascii(b"4k3/8/8/8/8/8/4P3/4K3 w - - 0 1")
            .unwrap()
            .into_position(CastlingMode::Standard)
            .unwrap();
        let config = EvaluationConfig {
            pawn_mobility_weight: 100,
            ..EvaluationConfig::default()
        };

        assert_ne!(
            mobility_evaluation_with_config(&position, &EvaluationConfig::default()),
            mobility_evaluation_with_config(&position, &config)
        );
    }
}
