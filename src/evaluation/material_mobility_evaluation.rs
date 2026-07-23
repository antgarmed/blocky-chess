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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialMobilityConfig {
    pub mobility_weight: Value,
    pub pawn_mobility_weight: Value,
    pub knight_mobility_weight: Value,
    pub bishop_mobility_weight: Value,
    pub rook_mobility_weight: Value,
    pub queen_mobility_weight: Value,
    pub king_mobility_weight: Value,
}

impl Default for MaterialMobilityConfig {
    fn default() -> Self {
        Self {
            mobility_weight: MOBILITY_WEIGHT,
            pawn_mobility_weight: PAWN_MOBILITY_WEIGHT_HUNDREDTHS,
            knight_mobility_weight: KNIGHT_MOBILITY_WEIGHT_HUNDREDTHS,
            bishop_mobility_weight: BISHOP_MOBILITY_WEIGHT_HUNDREDTHS,
            rook_mobility_weight: ROOK_MOBILITY_WEIGHT_HUNDREDTHS,
            queen_mobility_weight: QUEEN_MOBILITY_WEIGHT_HUNDREDTHS,
            king_mobility_weight: KING_MOBILITY_WEIGHT_HUNDREDTHS,
        }
    }
}

pub fn material_mobility_evaluation_with_config(
    position: &Chess,
    config: &MaterialMobilityConfig,
) -> Value {
    let outcome = position.outcome();
    if outcome.is_known() {
        return evaluate_outcome(&outcome);
    }

    let material_value = material_evaluation(position);
    let mobility_value =
        get_white_mobility(position, config) - get_black_mobility(position, config);

    material_value + config.mobility_weight * mobility_value / MOBILITY_WEIGHT_SCALE
}

const fn mobility_weight(role: Role, config: &MaterialMobilityConfig) -> Value {
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
    config: &MaterialMobilityConfig,
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

fn get_weighted_number_of_moves_for_white(
    position: &Chess,
    config: &MaterialMobilityConfig,
) -> Value {
    get_weighted_number_of_moves_for_color(position, Color::White, config)
}

fn get_weighted_number_of_moves_for_black(
    position: &Chess,
    config: &MaterialMobilityConfig,
) -> Value {
    get_weighted_number_of_moves_for_color(position, Color::Black, config)
}

fn get_white_mobility(position: &Chess, config: &MaterialMobilityConfig) -> Value {
    get_weighted_number_of_moves_for_white(position, config)
}

fn get_black_mobility(position: &Chess, config: &MaterialMobilityConfig) -> Value {
    get_weighted_number_of_moves_for_black(position, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::{fen::Fen, CastlingMode};

    #[test]
    fn default_config_preserves_the_existing_evaluation() {
        let position: Chess = Fen::from_ascii(b"4k3/8/8/8/8/8/4P3/4K3 w - - 0 1")
            .unwrap()
            .into_position(CastlingMode::Standard)
            .unwrap();

        assert_eq!(
            material_mobility_evaluation_with_config(&position, &MaterialMobilityConfig::default(),),
            material_mobility_evaluation_with_config(&position, &MaterialMobilityConfig::default())
        );
    }

    #[test]
    fn piece_mobility_weights_are_read_from_the_configuration() {
        let position: Chess = Fen::from_ascii(b"4k3/8/8/8/8/8/4P3/4K3 w - - 0 1")
            .unwrap()
            .into_position(CastlingMode::Standard)
            .unwrap();
        let config = MaterialMobilityConfig {
            pawn_mobility_weight: 100,
            ..MaterialMobilityConfig::default()
        };

        assert_ne!(
            material_mobility_evaluation_with_config(&position, &MaterialMobilityConfig::default(),),
            material_mobility_evaluation_with_config(&position, &config)
        );
    }
}
