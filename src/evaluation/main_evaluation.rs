use shakmaty::{Chess, Position};

use crate::search::Value;

use super::{
    evaluate_outcome, king_safety_evaluation::king_safety_evaluation,
    material_evaluation::material_evaluation,
    material_mobility_evaluation::mobility_evaluation_with_config, EvaluationConfig,
};

pub fn main_evaluation(position: &Chess, config: &EvaluationConfig) -> Value {
    let outcome = position.outcome();
    if outcome.is_known() {
        return evaluate_outcome(&outcome);
    }

    material_evaluation(position)
        + mobility_evaluation_with_config(position, config)
        + config.king_safety_weight * king_safety_evaluation(position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::{fen::Fen, CastlingMode};

    #[test]
    fn king_safety_weight_changes_main_evaluation() {
        let position: Chess = Fen::from_ascii(b"4k3/8/8/8/8/8/P7/6K1 w - - 0 1")
            .unwrap()
            .into_position(CastlingMode::Standard)
            .unwrap();
        let without_king_safety = EvaluationConfig {
            king_safety_weight: 0,
            ..EvaluationConfig::default()
        };

        assert_eq!(
            main_evaluation(&position, &EvaluationConfig::default())
                - main_evaluation(&position, &without_king_safety),
            EvaluationConfig::default().king_safety_weight
        );
    }
}
