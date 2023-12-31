use shakmaty::{Chess, MoveList, Position};

pub fn basic_movegen(position: &Chess) -> MoveList {
    position.legal_moves()
}
