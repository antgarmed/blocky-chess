use crate::{search::Value, utils::consts::MATE_VALUE};
use shakmaty::{Chess, Position};

pub fn zero_evalution(position: &Chess) -> Value {
    if position.outcome().is_some() {
        if position.outcome().unwrap().winner().unwrap().is_white() {
            return MATE_VALUE;
        } else {
            return -MATE_VALUE;
        }
    }

    return 0;
}
