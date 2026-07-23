use crate::engine::Engine;
use crate::evaluation::{main_evaluation::main_evaluation, EvaluationConfig};
use crate::movegen::basic_movegen::basic_movegen;
use crate::search::alpha_beta_iterative_deepening::AlphaBetaIterativeDeepeningSearch;
use crate::search::{SearchConfig, SearchLimits, SearchResult};
use shakmaty::{CastlingMode, Color, Position};
use std::fmt::Display;
use std::io::{self, BufRead, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use vampirc_uci::{parse_one, UciMessage, UciOptionConfig, UciSearchControl, UciTimeControl};

struct ActiveSearch {
    stop: Arc<AtomicBool>,
    worker: JoinHandle<()>,
}

impl ActiveSearch {
    fn stop_and_join(self) {
        self.stop.store(true, Ordering::Relaxed);
        if self.worker.join().is_err() {
            eprintln!("Search worker panicked");
        }
    }
}

pub fn start() {
    if let Err(error) = run_uci(io::stdin().lock(), Arc::new(Mutex::new(io::stdout()))) {
        eprintln!("UCI I/O error: {error}");
    }
}

fn run_uci<R, W>(reader: R, output: Arc<Mutex<W>>) -> io::Result<()>
where
    R: BufRead,
    W: Write + Send + 'static,
{
    let mut engine = get_engine();
    let mut evaluation_config = EvaluationConfig::default();
    let mut active_search: Option<ActiveSearch> = None;

    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("Error reading input: {}", error);
                continue;
            }
        };

        match parse_one(&line) {
            UciMessage::Uci => {
                write_line(
                    &output,
                    UciMessage::Id {
                        name: Some(engine.get_full_name()),
                        author: None,
                    },
                )?;
                write_line(
                    &output,
                    UciMessage::Id {
                        name: None,
                        author: Some(engine.get_author()),
                    },
                )?;
                for option in evaluation_options(&evaluation_config) {
                    write_line(&output, UciMessage::Option(option))?;
                }
                write_line(&output, UciMessage::UciOk)?;
            }
            UciMessage::SetOption { name, value } => {
                stop_active(&mut active_search);
                if apply_evaluation_option(&name, value.as_deref(), &mut evaluation_config) {
                    engine.set_evaluation_config(evaluation_config);
                }
            }
            UciMessage::IsReady => write_line(&output, UciMessage::ReadyOk)?,
            UciMessage::Stop => stop_active(&mut active_search),
            UciMessage::UciNewGame => stop_active(&mut active_search),
            UciMessage::Position {
                startpos,
                fen,
                moves,
            } => {
                stop_active(&mut active_search);
                let fen = fen.map(|fen| fen.to_string());
                let moves = moves
                    .into_iter()
                    .map(|chess_move| chess_move.to_string())
                    .collect::<Vec<_>>();
                let result = if startpos {
                    engine.set_uci_position(None, &moves)
                } else if let Some(fen) = fen.as_deref() {
                    engine.set_uci_position(Some(fen), &moves)
                } else {
                    eprintln!("Ignoring position command without startpos or FEN");
                    continue;
                };
                if let Err(error) = result {
                    eprintln!("Ignoring invalid position command: {error}");
                }
            }
            UciMessage::Go {
                time_control,
                search_control,
            } => {
                stop_active(&mut active_search);
                active_search = Some(start_search(
                    &engine,
                    time_control,
                    search_control,
                    Arc::clone(&output),
                ));
            }
            UciMessage::Quit => {
                stop_active(&mut active_search);
                break;
            }
            _ => {}
        }
    }
    stop_active(&mut active_search);
    Ok(())
}

fn stop_active(active: &mut Option<ActiveSearch>) {
    if let Some(search) = active.take() {
        search.stop_and_join();
    }
}

fn get_engine() -> Engine {
    let config = EvaluationConfig::default();
    Engine::new(Box::new(AlphaBetaIterativeDeepeningSearch::new(
        SearchConfig {
            evaluation_function: main_evaluation,
            move_generator: basic_movegen,
            evaluation_config: Arc::new(std::sync::RwLock::new(config)),
        },
    )))
}

fn evaluation_options(config: &EvaluationConfig) -> [UciOptionConfig; 8] {
    [
        spin_option("MobilityWeight", config.mobility_weight),
        spin_option("PawnMobilityWeight", config.pawn_mobility_weight),
        spin_option("KnightMobilityWeight", config.knight_mobility_weight),
        spin_option("BishopMobilityWeight", config.bishop_mobility_weight),
        spin_option("RookMobilityWeight", config.rook_mobility_weight),
        spin_option("QueenMobilityWeight", config.queen_mobility_weight),
        spin_option("KingMobilityWeight", config.king_mobility_weight),
        spin_option("KingSafetyWeight", config.king_safety_weight),
    ]
}

fn spin_option(name: &str, default: i64) -> UciOptionConfig {
    UciOptionConfig::Spin {
        name: name.to_owned(),
        default: Some(default),
        min: Some(0),
        max: Some(100),
    }
}

fn apply_evaluation_option(name: &str, value: Option<&str>, config: &mut EvaluationConfig) -> bool {
    let Some(value) = value.and_then(|value| value.parse::<i64>().ok()) else {
        return false;
    };
    if !(0..=100).contains(&value) {
        return false;
    }

    let target = match name {
        name if name.eq_ignore_ascii_case("MobilityWeight") => &mut config.mobility_weight,
        name if name.eq_ignore_ascii_case("PawnMobilityWeight") => &mut config.pawn_mobility_weight,
        name if name.eq_ignore_ascii_case("KnightMobilityWeight") => {
            &mut config.knight_mobility_weight
        }
        name if name.eq_ignore_ascii_case("BishopMobilityWeight") => {
            &mut config.bishop_mobility_weight
        }
        name if name.eq_ignore_ascii_case("RookMobilityWeight") => &mut config.rook_mobility_weight,
        name if name.eq_ignore_ascii_case("QueenMobilityWeight") => {
            &mut config.queen_mobility_weight
        }
        name if name.eq_ignore_ascii_case("KingMobilityWeight") => &mut config.king_mobility_weight,
        name if name.eq_ignore_ascii_case("KingSafetyWeight") => &mut config.king_safety_weight,
        _ => return false,
    };
    *target = value;
    true
}

fn requested_depth(search_control: Option<UciSearchControl>) -> Option<usize> {
    search_control
        .and_then(|control| control.depth)
        .map(|depth| depth as usize)
        .filter(|depth| *depth > 0)
}

fn allocated_time(time_control: Option<&UciTimeControl>, turn: Color) -> Option<Duration> {
    match time_control? {
        UciTimeControl::MoveTime(time) => {
            let millis = time.num_milliseconds().max(1) as u64;
            Some(Duration::from_millis(millis.saturating_mul(9) / 10))
        }
        UciTimeControl::TimeLeft {
            white_time,
            black_time,
            white_increment,
            black_increment,
            moves_to_go,
        } => {
            let time = if turn.is_white() {
                white_time.as_ref()
            } else {
                black_time.as_ref()
            }?;
            let increment = if turn.is_white() {
                white_increment.as_ref()
            } else {
                black_increment.as_ref()
            }
            .map(|duration| duration.num_milliseconds().max(0) as u64)
            .unwrap_or(0);
            let clock = time.num_milliseconds().max(0) as u64;
            let moves = u64::from(moves_to_go.unwrap_or(30).max(1));
            let target = clock / moves + increment;
            Some(Duration::from_millis(target.saturating_mul(8) / 10).max(Duration::from_millis(1)))
        }
        UciTimeControl::Infinite | UciTimeControl::Ponder => None,
    }
}

fn start_search(
    engine: &Engine,
    time_control: Option<UciTimeControl>,
    search_control: Option<UciSearchControl>,
    output: Arc<Mutex<impl Write + Send + 'static>>,
) -> ActiveSearch {
    let depth = requested_depth(search_control);
    let (position, search, turn) = engine.search_snapshot();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker = thread::spawn(move || {
        let deadline =
            allocated_time(time_control.as_ref(), turn).map(|budget| Instant::now() + budget);
        let limits = SearchLimits {
            depth,
            deadline,
            stop: &worker_stop,
        };
        let mut on_iteration = |completed_depth: usize, result: &SearchResult| {
            if let Err(error) = emit_info(completed_depth, result, turn, &output) {
                eprintln!("UCI search info output error: {error}");
            }
        };
        let result = search.search_with_limits(&position, &limits, &mut on_iteration);
        if let Err(error) = emit_bestmove(result.as_ref(), &position, &output) {
            eprintln!("UCI search output error: {error}");
        }
    });
    ActiveSearch { stop, worker }
}

fn format_score(search_result: &SearchResult, turn: Color) -> String {
    match search_result.get_mate_in() {
        Some(mate_in) => {
            let wins = (turn.is_white() && search_result.is_white_winning())
                || (turn.is_black() && search_result.is_black_winning());
            format!("mate {}{}", if wins { "" } else { "-" }, mate_in)
        }
        None => format!("cp {}", search_result.value),
    }
}

fn emit_info<W: Write>(
    completed_depth: usize,
    result: &SearchResult,
    turn: Color,
    output: &Arc<Mutex<W>>,
) -> io::Result<()> {
    let pv = result
        .principal_variation
        .iter()
        .map(|chess_move| chess_move.to_uci(CastlingMode::Standard).to_string())
        .collect::<Vec<_>>()
        .join(" ");
    write_line(
        output,
        format!(
            "info depth {} score {} pv {}",
            completed_depth,
            format_score(result, turn),
            pv
        ),
    )
}

fn emit_bestmove<W: Write>(
    result: Option<&(usize, SearchResult)>,
    position: &shakmaty::Chess,
    output: &Arc<Mutex<W>>,
) -> io::Result<()> {
    if let Some((_completed_depth, result)) = result {
        if let Some(best_move) = result.principal_variation.first() {
            write_line(
                output,
                format!("bestmove {}", best_move.to_uci(CastlingMode::Standard)),
            )?;
            return Ok(());
        }
    }

    // If stop arrives before depth one completes, UCI still requires exactly one response.
    if let Some(chess_move) = position.legal_moves().first() {
        write_line(
            output,
            format!("bestmove {}", chess_move.to_uci(CastlingMode::Standard)),
        )?;
    } else {
        write_line(output, "bestmove 0000")?;
    }
    Ok(())
}

fn write_line<W: Write>(output: &Arc<Mutex<W>>, line: impl Display) -> io::Result<()> {
    let mut output = output
        .lock()
        .map_err(|_| io::Error::other("UCI output lock poisoned"))?;
    writeln!(output, "{line}")?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::consts::MATE_VALUE;
    use std::io::Cursor;

    #[derive(Default)]
    struct FlushTrackingWriter {
        bytes: Vec<u8>,
        flushes: usize,
    }

    impl Write for FlushTrackingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }

    fn run_commands(commands: &str) -> String {
        let output = Arc::new(Mutex::new(Vec::new()));
        run_uci(Cursor::new(commands), Arc::clone(&output)).unwrap();
        let bytes = output.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn engine_creation_exposes_identity() {
        let engine = get_engine();
        assert!(!engine.get_full_name().is_empty());
        assert!(!engine.get_author().is_empty());
    }

    #[test]
    fn engine_is_silent_until_uci_command() {
        assert!(run_commands("quit\n").is_empty());
    }

    #[test]
    fn protocol_responses_are_flushed() {
        let output = Arc::new(Mutex::new(FlushTrackingWriter::default()));
        run_uci(Cursor::new("uci\nisready\nquit\n"), Arc::clone(&output)).unwrap();
        let output = output.lock().unwrap();

        assert_eq!(output.flushes, 12);
        assert_eq!(
            String::from_utf8(output.bytes.clone()).unwrap(),
            "id name Blocky 0.1.0\nid author antgarmed\noption name MobilityWeight type spin default 10 min 0 max 100\noption name PawnMobilityWeight type spin default 5 min 0 max 100\noption name KnightMobilityWeight type spin default 30 min 0 max 100\noption name BishopMobilityWeight type spin default 30 min 0 max 100\noption name RookMobilityWeight type spin default 20 min 0 max 100\noption name QueenMobilityWeight type spin default 10 min 0 max 100\noption name KingMobilityWeight type spin default 5 min 0 max 100\noption name KingSafetyWeight type spin default 50 min 0 max 100\nuciok\nreadyok\n"
        );
    }

    #[test]
    fn uci_announces_and_applies_evaluation_options() {
        let output = run_commands("uci\nsetoption name QueenMobilityWeight value 42\nquit\n");

        assert!(
            output.contains("option name QueenMobilityWeight type spin default 10 min 0 max 100")
        );
    }

    #[test]
    fn invalid_evaluation_option_is_ignored() {
        let mut config = EvaluationConfig::default();

        assert!(!apply_evaluation_option(
            "MobilityWeight",
            Some("101"),
            &mut config
        ));
        assert_eq!(config.mobility_weight, 10);
        assert!(!apply_evaluation_option("Unknown", Some("20"), &mut config));
    }

    #[test]
    fn absent_depth_is_unbounded_and_explicit_depth_is_supported() {
        assert_eq!(requested_depth(None), None);
        assert_eq!(
            requested_depth(Some(UciSearchControl {
                depth: Some(4),
                ..Default::default()
            })),
            Some(4)
        );
    }

    #[test]
    fn time_left_allocates_clock_time_by_moves_to_go() {
        let UciMessage::Go {
            time_control: Some(time_control),
            ..
        } = parse_one("go wtime 300000 btime 300000 movestogo 40")
        else {
            panic!("expected time control");
        };

        assert_eq!(
            allocated_time(Some(&time_control), Color::White),
            Some(Duration::from_millis(6_000))
        );
    }

    #[test]
    fn score_mate_is_relative_to_side_to_move() {
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

    #[test]
    fn engine_applies_uci_move_and_changes_turn() {
        let mut engine = get_engine();
        engine.set_uci_position(None, ["e2e4"]).unwrap();

        assert_eq!(engine.turn(), Color::Black);
    }

    #[test]
    fn invalid_position_update_preserves_previous_position() {
        let mut engine = get_engine();
        engine.set_uci_position(None, ["e2e4"]).unwrap();
        let previous_position = engine.search_snapshot().0;

        assert!(engine.set_uci_position(None, ["e2e5"]).is_err());
        assert_eq!(engine.search_snapshot().0, previous_position);
    }

    #[test]
    fn engine_rejects_invalid_fen_and_malformed_promotion() {
        let mut engine = get_engine();

        assert!(engine
            .set_uci_position(Some("not-a-fen"), std::iter::empty::<&str>())
            .is_err());
        assert!(engine.set_uci_position(None, ["e7e8x"]).is_err());
    }

    #[test]
    fn invalid_position_commands_do_not_terminate_uci_session() {
        let commands = [
            "position fen 8/8/8/8/8/8/8/8 w - - 0 1",
            "position startpos moves e2e5",
            "position startpos moves e7e8x",
            "position startpos moves e2e4 e7e5 e1e3",
        ];

        for command in commands {
            let output = run_commands(&format!("{command}\nisready\nquit\n"));
            assert_eq!(output, "readyok\n", "failed for command: {command}");
        }
    }

    #[test]
    fn stop_interrupts_search_after_readyok_and_emits_one_bestmove() {
        let output = run_commands("position startpos\ngo depth 255\nisready\nstop\nquit\n");
        let ready = output.find("readyok").unwrap();
        let bestmove = output.find("bestmove ").unwrap();

        assert!(ready < bestmove, "unexpected UCI output:\n{output}");
        assert_eq!(output.matches("bestmove ").count(), 1, "{output}");
    }
}
