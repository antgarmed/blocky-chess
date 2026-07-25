use shakmaty::{Chess, Color, KnownOutcome, MoveList, Outcome, Piece, Position, Role};

use crate::{search::Value, utils::consts::MATE_VALUE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvaluationConfig {
    pub pawn_value: Value,
    pub knight_value: Value,
    pub bishop_value: Value,
    pub rook_value: Value,
    pub queen_value: Value,
    pub mobility_weight: Value,
    pub pawn_mobility_weight: Value,
    pub knight_mobility_weight: Value,
    pub bishop_mobility_weight: Value,
    pub rook_mobility_weight: Value,
    pub queen_mobility_weight: Value,
    pub king_mobility_weight: Value,
    pub king_safety_weight: Value,
}

impl EvaluationConfig {
    pub(crate) fn from_normalized_genes(genes: [f64; 12]) -> Option<Self> {
        if genes.iter().any(|gene| !gene.is_finite() || *gene < 0.0)
            || genes.iter().all(|gene| *gene == 0.0)
        {
            return None;
        }
        let quantize = |gene: f64| (gene * 4_000.0).round() as Value;
        Some(Self {
            pawn_value: quantize(genes[0]),
            knight_value: quantize(genes[1]),
            bishop_value: quantize(genes[2]),
            rook_value: quantize(genes[3]),
            queen_value: quantize(genes[4]),
            mobility_weight: 100,
            pawn_mobility_weight: quantize(genes[5]),
            knight_mobility_weight: quantize(genes[6]),
            bishop_mobility_weight: quantize(genes[7]),
            rook_mobility_weight: quantize(genes[8]),
            queen_mobility_weight: quantize(genes[9]),
            king_mobility_weight: quantize(genes[10]),
            king_safety_weight: quantize(genes[11]),
        })
    }
}

impl Default for EvaluationConfig {
    fn default() -> Self {
        Self {
            pawn_value: 100,
            knight_value: 300,
            bishop_value: 300,
            rook_value: 500,
            queen_value: 900,
            mobility_weight: 10,
            pawn_mobility_weight: 5,
            knight_mobility_weight: 30,
            bishop_mobility_weight: 30,
            rook_mobility_weight: 20,
            queen_mobility_weight: 10,
            king_mobility_weight: 5,
            king_safety_weight: 50,
        }
    }
}

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

pub mod king_safety_evaluation;
pub mod main_evaluation;
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
