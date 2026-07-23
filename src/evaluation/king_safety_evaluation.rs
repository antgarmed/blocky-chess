use shakmaty::{Chess, Color, Position, Square};

use crate::search::Value;

pub fn king_safety_evaluation(position: &Chess) -> Value {
    let white_castled = matches!(
        position.board().king_of(Color::White),
        Some(Square::C1 | Square::G1)
    );
    let black_castled = matches!(
        position.board().king_of(Color::Black),
        Some(Square::C8 | Square::G8)
    );

    white_castled as Value - black_castled as Value
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::{fen::Fen, CastlingMode};

    fn position(fen: &str) -> Chess {
        Fen::from_ascii(fen.as_bytes())
            .unwrap()
            .into_position(CastlingMode::Standard)
            .unwrap()
    }

    #[test]
    fn rewards_castled_white_king() {
        assert_eq!(
            king_safety_evaluation(&position("4k3/8/8/8/8/8/8/6K1 w - - 0 1")),
            1
        );
    }

    #[test]
    fn rewards_castled_black_king() {
        assert_eq!(
            king_safety_evaluation(&position("6k1/8/8/8/8/8/8/4K3 w - - 0 1")),
            -1
        );
    }

    #[test]
    fn is_balanced_when_both_kings_are_castled() {
        assert_eq!(
            king_safety_evaluation(&position("6k1/8/8/8/8/8/8/6K1 w - - 0 1")),
            0
        );
    }
}
