use shakmaty::{Chess, Color, Outcome, Piece, Position, Role};

use crate::{search::Value, utils::consts::MATE_VALUE};

fn evaluate_outcome(outcome: &Outcome) -> Value {
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

fn get_number_of_white_pawns(position: &Chess) -> usize {
    position
        .board()
        .by_piece(Piece {
            color: Color::White,
            role: Role::Pawn,
        })
        .count()
}

fn get_number_of_black_pawns(position: &Chess) -> usize {
    position
        .board()
        .by_piece(Piece {
            color: Color::Black,
            role: Role::Pawn,
        })
        .count()
}

fn get_number_of_white_knights(position: &Chess) -> usize {
    position
        .board()
        .by_piece(Piece {
            color: Color::White,
            role: Role::Knight,
        })
        .count()
}

fn get_number_of_black_knights(position: &Chess) -> usize {
    position
        .board()
        .by_piece(Piece {
            color: Color::Black,
            role: Role::Knight,
        })
        .count()
}

fn get_number_of_white_bishops(position: &Chess) -> usize {
    position
        .board()
        .by_piece(Piece {
            color: Color::White,
            role: Role::Bishop,
        })
        .count()
}

fn get_number_of_black_bishops(position: &Chess) -> usize {
    position
        .board()
        .by_piece(Piece {
            color: Color::Black,
            role: Role::Bishop,
        })
        .count()
}

fn get_number_of_white_rooks(position: &Chess) -> usize {
    position
        .board()
        .by_piece(Piece {
            color: Color::White,
            role: Role::Rook,
        })
        .count()
}

fn get_number_of_black_rooks(position: &Chess) -> usize {
    position
        .board()
        .by_piece(Piece {
            color: Color::Black,
            role: Role::Rook,
        })
        .count()
}

fn get_number_of_white_queens(position: &Chess) -> usize {
    position
        .board()
        .by_piece(Piece {
            color: Color::White,
            role: Role::Queen,
        })
        .count()
}

fn get_number_of_black_queens(position: &Chess) -> usize {
    position
        .board()
        .by_piece(Piece {
            color: Color::Black,
            role: Role::Queen,
        })
        .count()
}

fn get_number_of_moves_for_white(position: &Chess) -> usize {
    if position.turn() == Color::White {
        position.clone().swap_turn().unwrap().legal_moves().len();
    }

    position.legal_moves().len()
}

fn get_number_of_moves_for_black(position: &Chess) -> usize {
    if position.turn() == Color::White {
        position.clone().swap_turn().unwrap().legal_moves().len();
    }

    position.legal_moves().len()
}

pub mod material_evaluation;
pub mod material_mobility_evaluation;
pub mod zero_evaluation;

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::fen::Fen;
    use shakmaty::CastlingMode;

    #[test]
    fn test_search_returns_result_when_depth_is_1() {
        let fen: Fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
            .parse()
            .unwrap();
        let position: Chess = fen.into_position(CastlingMode::Standard).unwrap();

        let result = get_number_of_moves_for_white(&position);

        assert_eq!(result, 20);
    }
}
