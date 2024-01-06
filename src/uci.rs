use crate::engine::Engine;
use crate::evaluation::zero_evaluation::zero_evaluation;
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
        let message: UciMessage = parse_one(&line.unwrap());
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
        evaluation_function: zero_evaluation,
        move_generator: basic_movegen,
    }));

    Engine::new(search_algorithm)
}

fn handle_go(
    engine: &mut Engine,
    _time_control: Option<UciTimeControl>,
    search_control: Option<UciSearchControl>,
) {
    let mut depth = 6;

    if search_control.is_some() {
        depth = search_control.unwrap().depth.unwrap() as usize;
    }

    let search_result = engine.go(depth);

    let best_move = search_result.principal_variation.first().unwrap();
    let score = search_result.value;
    let pv = search_result
        .principal_variation
        .iter()
        .map(|m| m.to_uci(CastlingMode::Standard).to_string())
        .collect::<Vec<String>>()
        .join(" ");

    println!("info depth {} score {} pv {}", depth, score, pv);
    println!(
        "bestmove {}",
        best_move.to_uci(CastlingMode::Standard).to_string()
    );
}
