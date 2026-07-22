use crate::engine::Engine;
use crate::evaluation::material_mobility_evaluation::material_mobility_evaluation;
use crate::movegen::basic_movegen::basic_movegen;
use crate::search::alphabeta::AlphaBetaSearch;
use crate::search::SearchConfig;
use shakmaty::{CastlingMode, Color};
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

fn format_score(search_result: &crate::search::SearchResult, turn: Color) -> String {
    match search_result.get_mate_in() {
        Some(mate_in) => {
            let side_to_move_wins = (turn.is_white() && search_result.is_white_winning())
                || (turn.is_black() && search_result.is_black_winning());
            let sign = if side_to_move_wins { "" } else { "-" };
            format!("mate {}{}", sign, mate_in)
        }
        None => format!("cp {}", search_result.value),
    }
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
            let score = format_score(&search_result, engine.turn());
            let pv = search_result
                .principal_variation
                .iter()
                .map(|m| m.to_uci(CastlingMode::Standard).to_string())
                .collect::<Vec<String>>()
                .join(" ");

            println!("info depth {} score {} pv {}", depth, score, pv);
            println!("bestmove {}", best_move.to_uci(CastlingMode::Standard));
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
    use crate::search::SearchResult;
    use crate::utils::consts::MATE_VALUE;
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

    #[test]
    fn test_format_score_mate_is_relative_to_side_to_move() {
        let white_wins = SearchResult {
            value: MATE_VALUE - 1,
            principal_variation: Vec::new(),
        };
        let black_wins = SearchResult {
            value: 1 - MATE_VALUE,
            principal_variation: Vec::new(),
        };

        assert_eq!(format_score(&white_wins, Color::White), "mate 1");
        assert_eq!(format_score(&white_wins, Color::Black), "mate -1");
        assert_eq!(format_score(&black_wins, Color::Black), "mate 1");
        assert_eq!(format_score(&black_wins, Color::White), "mate -1");
    }
}
