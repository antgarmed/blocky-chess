use crate::engine::Engine;
use crate::evaluation::material_mobility_evaluation::material_mobility_evaluation;
use crate::movegen::basic_movegen::basic_movegen;
use crate::search::alphabeta::AlphaBetaSearch;
use crate::search::SearchConfig;
use shakmaty::CastlingMode;
use std::io::{self, BufRead};
use vampirc_uci::parse_one;
use vampirc_uci::UciMessage;
use vampirc_uci::UciSearchControl;
use vampirc_uci::UciTimeControl;

pub fn start() {
    let mut engine = get_engine();

    println!("{}", engine.get_full_name());

    for line in io::stdin().lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                continue;
            }
        };
        let message: UciMessage = parse_one(&line);
        match message {
            UciMessage::Uci => {
                let response = UciMessage::Id {
                    name: Some(engine.get_full_name()),
                    author: Some(engine.get_author()),
                };
                println!("{}", response);

                let response = UciMessage::UciOk;
                println!("{}", response);
            }
            UciMessage::IsReady => {
                let response = UciMessage::ReadyOk;
                println!("{}", response);
            }
            UciMessage::UciNewGame => {}
            UciMessage::Position {
                startpos,
                fen,
                moves,
            } => {
                if startpos {
                    engine.set_default_position();
                } else {
                    engine.set_position_from_fen(fen.unwrap().to_string().as_str());
                }

                for m in moves {
                    engine.make_uci_move(m.to_string().as_str());
                }
            }
            UciMessage::Go {
                time_control,
                search_control,
            } => {
                handle_go(&mut engine, time_control, search_control);
            }
            UciMessage::Quit => {
                break;
            }
            _ => {}
        }
    }
}

fn get_engine() -> Engine {
    let search_algorithm = Box::new(AlphaBetaSearch::new(SearchConfig {
        evaluation_function: material_mobility_evaluation,
        move_generator: basic_movegen,
    }));

    Engine::new(search_algorithm)
}

fn handle_go(
    engine: &mut Engine,
    _time_control: Option<UciTimeControl>,
    search_control: Option<UciSearchControl>,
) {
    let depth = search_control
        .and_then(|sc| sc.depth)
        .map(|d| d as usize)
        .unwrap_or(6);

    let search_result = engine.go(depth);

    match search_result.principal_variation.first() {
        Some(best_move) => {
            let score = search_result.value;
            let pv = search_result
                .principal_variation
                .iter()
                .map(|m| m.to_uci(CastlingMode::Standard).to_string())
                .collect::<Vec<String>>()
                .join(" ");

            println!("info depth {} score cp {} pv {}", depth, score, pv);
            println!(
                "bestmove {}",
                best_move.to_uci(CastlingMode::Standard).to_string()
            );
        }
        None => {
            println!("info string No legal moves found");
            println!("bestmove 0000"); // UCI protocol's way to indicate no move
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vampirc_uci::UciSearchControl;

    #[test]
    fn test_engine_creation() {
        let engine = get_engine();
        assert!(!engine.get_full_name().is_empty());
        assert!(!engine.get_author().is_empty());
    }

    #[test]
    fn test_handle_go_with_default_depth() {
        let mut engine = get_engine();
        engine.set_default_position();
        let search_control = Some(UciSearchControl {
            depth: Some(4),
            ..Default::default()
        });
        handle_go(&mut engine, None, search_control);
    }

    #[test]
    fn test_handle_go_with_custom_depth() {
        let mut engine = get_engine();
        engine.set_default_position();
        let search_control = Some(UciSearchControl {
            depth: Some(4),
            ..Default::default()
        });
        handle_go(&mut engine, None, search_control);
    }

    #[test]
    fn test_position_handling() {
        let mut engine = get_engine();
        engine.set_default_position();
        engine.make_uci_move("e2e4");
    }
}
