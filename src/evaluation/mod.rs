use shakmaty::{Chess, Color, KnownOutcome, MoveList, Outcome, Piece, Position, Role};

use crate::{search::Value, utils::consts::MATE_VALUE};

fn evaluate_outcome(outcome: &Outcome) -> Value {
    match outcome {
        Outcome::Known(KnownOutcome::Decisive { winner }) => match winner {
            Color::White => MATE_VALUE,
            Color::Black => -MATE_VALUE,
        },
        Outcome::Known(KnownOutcome::Draw) | Outcome::Unknown => 0,
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

#[cfg(test)]
fn get_number_of_moves_for_color(position: &Chess, color: Color) -> usize {
    get_legal_moves_for_color(position, color)
        .map(|moves| moves.len())
        .unwrap_or(0)
}

fn get_legal_moves_for_color(position: &Chess, color: Color) -> Option<MoveList> {
    if position.turn() == color {
        Some(position.legal_moves())
    } else {
        position
            .clone()
            .swap_turn()
            .ok()
            .map(|position| position.legal_moves())
    }
}

#[cfg(test)]
fn get_number_of_moves_for_white(position: &Chess) -> usize {
    get_number_of_moves_for_color(position, Color::White)
}

#[cfg(test)]
fn get_number_of_moves_for_black(position: &Chess) -> usize {
    get_number_of_moves_for_color(position, Color::Black)
}

pub mod material_evaluation;
pub mod material_mobility_evaluation;
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

    #[test]
    fn test_mobility_is_calculated_for_each_color() {
        let fen: Fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
            .parse()
            .unwrap();
        let position: Chess = fen.into_position(CastlingMode::Standard).unwrap();

        assert_eq!(get_number_of_moves_for_black(&position), 20);
        assert_eq!(get_number_of_moves_for_white(&position), 30);
    }
}
