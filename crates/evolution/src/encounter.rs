//! Paired games that neutralize color and opening variance.

use std::{collections::BTreeMap, error::Error, fmt, num::NonZeroUsize};

use blocky_chess::EvaluationConfig;

use crate::{
    genome::Genome,
    openings::Opening,
    pairing::{IndividualId, Pairing, Round, Score},
    self_play::{GameError, GameOutcome, GameRecord, SearchMoveSelector, SelfPlayGame},
    training::TrainingConfig,
};

pub trait GameRunner {
    type Error;

    fn play(
        &mut self,
        white: &Genome,
        black: &Genome,
        opening: &Opening,
        search_depth: usize,
        max_game_plies: usize,
    ) -> Result<GameRecord, Self::Error>;
}

/// Runs a game from exact evaluator configurations. This is kept separate from
/// [`GameRunner`] so external validation can benchmark the literal reference
/// configuration without changing the genome-based training path.
pub trait ConfiguredGameRunner {
    type Error;

    fn play_configured(
        &mut self,
        white: EvaluationConfig,
        black: EvaluationConfig,
        opening: &Opening,
        search_depth: usize,
        max_game_plies: usize,
    ) -> Result<GameRecord, Self::Error>;
}

#[derive(Clone, Copy, Default)]
pub struct ProductionGameRunner;

pub trait GameRunnerFactory {
    type Runner: GameRunner;

    fn create(&self) -> Self::Runner;
}

impl GameRunnerFactory for ProductionGameRunner {
    type Runner = Self;

    fn create(&self) -> Self::Runner {
        *self
    }
}

pub trait ConfiguredGameRunnerFactory {
    type Runner: ConfiguredGameRunner;

    fn create(&self) -> Self::Runner;
}

impl ConfiguredGameRunnerFactory for ProductionGameRunner {
    type Runner = Self;

    fn create(&self) -> Self::Runner {
        *self
    }
}

impl GameRunner for ProductionGameRunner {
    type Error = ProductionGameError;

    fn play(
        &mut self,
        white: &Genome,
        black: &Genome,
        opening: &Opening,
        search_depth: usize,
        max_game_plies: usize,
    ) -> Result<GameRecord, Self::Error> {
        self.play_configured(
            white.to_evaluation_config(),
            black.to_evaluation_config(),
            opening,
            search_depth,
            max_game_plies,
        )
    }
}

impl ConfiguredGameRunner for ProductionGameRunner {
    type Error = ProductionGameError;

    fn play_configured(
        &mut self,
        white: EvaluationConfig,
        black: EvaluationConfig,
        opening: &Opening,
        search_depth: usize,
        max_game_plies: usize,
    ) -> Result<GameRecord, Self::Error> {
        let white = SearchMoveSelector::alpha_beta(white, search_depth)?;
        let black = SearchMoveSelector::alpha_beta(black, search_depth)?;
        Ok(
            SelfPlayGame::from_position(opening.position.clone(), white, black, max_game_plies)
                .play()?,
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProductionGameError {
    Selector(crate::self_play::SearchMoveSelectorError),
    Game(GameError),
}

impl From<crate::self_play::SearchMoveSelectorError> for ProductionGameError {
    fn from(value: crate::self_play::SearchMoveSelectorError) -> Self {
        Self::Selector(value)
    }
}
impl From<GameError> for ProductionGameError {
    fn from(value: GameError) -> Self {
        Self::Game(value)
    }
}
impl fmt::Display for ProductionGameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for ProductionGameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Selector(source) => Some(source),
            Self::Game(source) => Some(source),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EncounterRecord {
    pub pairing: Pairing,
    pub opening: crate::openings::OpeningId,
    pub first_game: GameRecord,
    pub second_game: GameRecord,
    pub a_score: Score,
    pub b_score: Score,
}

#[derive(Clone, Debug)]
pub struct ConfiguredEncounterRecord {
    pub candidate: IndividualId,
    pub first_game: GameRecord,
    pub second_game: GameRecord,
    pub candidate_score: Score,
}

fn play_configured_encounter<R: ConfiguredGameRunner>(
    runner: &mut R,
    candidate: IndividualId,
    candidate_config: EvaluationConfig,
    opening: &Opening,
    config: &TrainingConfig,
) -> Result<ConfiguredEncounterRecord, R::Error> {
    let reference = EvaluationConfig::default();
    let first_game = runner.play_configured(
        candidate_config,
        reference,
        opening,
        config.search_depth(),
        config.max_game_plies(),
    )?;
    let second_game = runner.play_configured(
        reference,
        candidate_config,
        opening,
        config.search_depth(),
        config.max_game_plies(),
    )?;
    let candidate_score =
        Score(points_for_white(first_game.outcome) + points_for_black(second_game.outcome));
    Ok(ConfiguredEncounterRecord {
        candidate,
        first_game,
        second_game,
        candidate_score,
    })
}

pub fn play_encounter<R: GameRunner>(
    runner: &mut R,
    pairing: Pairing,
    a: &Genome,
    b: &Genome,
    opening: &Opening,
    config: &TrainingConfig,
) -> Result<EncounterRecord, R::Error> {
    let first_game = runner.play(
        a,
        b,
        opening,
        config.search_depth(),
        config.max_game_plies(),
    )?;
    let second_game = runner.play(
        b,
        a,
        opening,
        config.search_depth(),
        config.max_game_plies(),
    )?;
    let a_score =
        Score(points_for_white(first_game.outcome) + points_for_black(second_game.outcome));
    let b_score = Score(4 - a_score.0);
    Ok(EncounterRecord {
        pairing,
        opening: opening.id,
        first_game,
        second_game,
        a_score,
        b_score,
    })
}

/// Executes every pairing in a round with the round's single shared opening.
pub fn play_round<R: GameRunner>(
    runner: &mut R,
    round: &Round,
    population: &BTreeMap<IndividualId, Genome>,
    opening: &Opening,
    config: &TrainingConfig,
) -> Result<Vec<EncounterRecord>, RoundExecutionError<R::Error>> {
    if round.opening != opening.id {
        return Err(RoundExecutionError::OpeningMismatch {
            scheduled: round.opening,
            supplied: opening.id,
        });
    }

    round
        .pairings
        .iter()
        .map(|pairing| {
            let a = population
                .get(&pairing.a)
                .ok_or(RoundExecutionError::MissingIndividual(pairing.a))?;
            let b = population
                .get(&pairing.b)
                .ok_or(RoundExecutionError::MissingIndividual(pairing.b))?;
            play_encounter(runner, *pairing, a, b, opening, config)
                .map_err(RoundExecutionError::Game)
        })
        .collect()
}

pub trait RoundExecutor {
    type Error;

    fn play_round(
        &mut self,
        round: &Round,
        population: &BTreeMap<IndividualId, Genome>,
        opening: &Opening,
        config: &TrainingConfig,
    ) -> Result<Vec<EncounterRecord>, RoundExecutionError<Self::Error>>;
}

pub trait DefaultAnchorRoundExecutor: RoundExecutor {
    fn play_default_anchor_round(
        &mut self,
        candidates: &[(IndividualId, EvaluationConfig)],
        opening: &Opening,
        config: &TrainingConfig,
    ) -> Result<Vec<ConfiguredEncounterRecord>, RoundExecutionError<Self::Error>>;
}

pub struct SequentialRoundExecutor<R> {
    runner: R,
}

impl<R> SequentialRoundExecutor<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    pub const fn runner(&self) -> &R {
        &self.runner
    }
}

impl<R: GameRunner> RoundExecutor for SequentialRoundExecutor<R> {
    type Error = R::Error;

    fn play_round(
        &mut self,
        round: &Round,
        population: &BTreeMap<IndividualId, Genome>,
        opening: &Opening,
        config: &TrainingConfig,
    ) -> Result<Vec<EncounterRecord>, RoundExecutionError<Self::Error>> {
        play_round(&mut self.runner, round, population, opening, config)
    }
}

impl<R> DefaultAnchorRoundExecutor for SequentialRoundExecutor<R>
where
    R: GameRunner + ConfiguredGameRunner<Error = <R as GameRunner>::Error>,
{
    fn play_default_anchor_round(
        &mut self,
        candidates: &[(IndividualId, EvaluationConfig)],
        opening: &Opening,
        config: &TrainingConfig,
    ) -> Result<Vec<ConfiguredEncounterRecord>, RoundExecutionError<Self::Error>> {
        candidates
            .iter()
            .map(|(id, candidate)| {
                play_configured_encounter(&mut self.runner, *id, *candidate, opening, config)
                    .map_err(RoundExecutionError::Game)
            })
            .collect()
    }
}

pub struct ParallelRoundExecutor<F> {
    factory: F,
    workers: NonZeroUsize,
}

impl<F> ParallelRoundExecutor<F> {
    pub fn new(factory: F, workers: NonZeroUsize) -> Self {
        Self { factory, workers }
    }
}

impl<F> RoundExecutor for ParallelRoundExecutor<F>
where
    F: GameRunnerFactory + Sync,
    F::Runner: Send,
    <F::Runner as GameRunner>::Error: Send,
{
    type Error = <F::Runner as GameRunner>::Error;

    fn play_round(
        &mut self,
        round: &Round,
        population: &BTreeMap<IndividualId, Genome>,
        opening: &Opening,
        config: &TrainingConfig,
    ) -> Result<Vec<EncounterRecord>, RoundExecutionError<Self::Error>> {
        if round.opening != opening.id {
            return Err(RoundExecutionError::OpeningMismatch {
                scheduled: round.opening,
                supplied: opening.id,
            });
        }
        for pairing in &round.pairings {
            if !population.contains_key(&pairing.a) {
                return Err(RoundExecutionError::MissingIndividual(pairing.a));
            }
            if !population.contains_key(&pairing.b) {
                return Err(RoundExecutionError::MissingIndividual(pairing.b));
            }
        }

        let worker_count = self.workers.get().min(round.pairings.len());
        if worker_count <= 1 {
            let mut runner = self.factory.create();
            return play_round(&mut runner, round, population, opening, config);
        }

        let worker_results = std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(worker_count);
            for worker in 0..worker_count {
                let factory = &self.factory;
                handles.push(scope.spawn(move || {
                    let mut runner = factory.create();
                    round
                        .pairings
                        .iter()
                        .enumerate()
                        .skip(worker)
                        .step_by(worker_count)
                        .map(|(index, pairing)| {
                            let a = population
                                .get(&pairing.a)
                                .expect("population was checked before dispatch");
                            let b = population
                                .get(&pairing.b)
                                .expect("population was checked before dispatch");
                            (
                                index,
                                play_encounter(&mut runner, *pairing, a, b, opening, config),
                            )
                        })
                        .collect::<Vec<_>>()
                }));
            }
            handles
                .into_iter()
                .map(|handle| handle.join())
                .collect::<Vec<_>>()
        });

        let mut ordered = (0..round.pairings.len()).map(|_| None).collect::<Vec<_>>();
        for worker_result in worker_results {
            for (index, result) in worker_result.map_err(|_| RoundExecutionError::WorkerPanic)? {
                ordered[index] = Some(result);
            }
        }
        ordered
            .into_iter()
            .map(|result| {
                result
                    .expect("every dispatched pairing must produce one result")
                    .map_err(RoundExecutionError::Game)
            })
            .collect()
    }
}

impl<F> DefaultAnchorRoundExecutor for ParallelRoundExecutor<F>
where
    F: GameRunnerFactory
        + ConfiguredGameRunnerFactory<Runner = <F as GameRunnerFactory>::Runner>
        + Sync,
    <F as GameRunnerFactory>::Runner: Send
        + ConfiguredGameRunner<Error = <<F as GameRunnerFactory>::Runner as GameRunner>::Error>,
    <<F as GameRunnerFactory>::Runner as GameRunner>::Error: Send,
{
    fn play_default_anchor_round(
        &mut self,
        candidates: &[(IndividualId, EvaluationConfig)],
        opening: &Opening,
        config: &TrainingConfig,
    ) -> Result<Vec<ConfiguredEncounterRecord>, RoundExecutionError<Self::Error>> {
        let worker_count = self.workers.get().min(candidates.len());
        if worker_count <= 1 {
            let mut runner = GameRunnerFactory::create(&self.factory);
            return candidates
                .iter()
                .map(|(id, candidate)| {
                    play_configured_encounter(&mut runner, *id, *candidate, opening, config)
                        .map_err(RoundExecutionError::Game)
                })
                .collect();
        }
        let factory = &self.factory;
        let worker_results = std::thread::scope(|scope| {
            (0..worker_count)
                .map(|worker| {
                    scope.spawn(move || {
                        let mut runner = GameRunnerFactory::create(factory);
                        candidates
                            .iter()
                            .enumerate()
                            .skip(worker)
                            .step_by(worker_count)
                            .map(|(index, (id, candidate))| {
                                (
                                    index,
                                    play_configured_encounter(
                                        &mut runner,
                                        *id,
                                        *candidate,
                                        opening,
                                        config,
                                    ),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join())
                .collect::<Vec<_>>()
        });
        let mut ordered = (0..candidates.len()).map(|_| None).collect::<Vec<_>>();
        for worker_result in worker_results {
            for (index, result) in worker_result.map_err(|_| RoundExecutionError::WorkerPanic)? {
                ordered[index] = Some(result);
            }
        }
        ordered
            .into_iter()
            .map(|result| {
                result
                    .expect("every anchor candidate must produce one result")
                    .map_err(RoundExecutionError::Game)
            })
            .collect()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RoundExecutionError<E> {
    OpeningMismatch {
        scheduled: crate::openings::OpeningId,
        supplied: crate::openings::OpeningId,
    },
    MissingIndividual(IndividualId),
    Game(E),
    WorkerPanic,
}

impl<E: fmt::Display> fmt::Display for RoundExecutionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpeningMismatch {
                scheduled,
                supplied,
            } => write!(
                formatter,
                "round expects opening {scheduled:?}, but {supplied:?} was supplied"
            ),
            Self::MissingIndividual(individual) => {
                write!(
                    formatter,
                    "round references missing individual {individual:?}"
                )
            }
            Self::Game(source) => write!(formatter, "game execution failed: {source}"),
            Self::WorkerPanic => formatter.write_str("parallel game worker panicked"),
        }
    }
}

impl<E: Error + 'static> Error for RoundExecutionError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Game(source) => Some(source),
            Self::OpeningMismatch { .. } | Self::MissingIndividual(_) | Self::WorkerPanic => None,
        }
    }
}

fn points_for_white(outcome: GameOutcome) -> u32 {
    match outcome {
        GameOutcome::WhiteWin => 2,
        GameOutcome::BlackWin => 0,
        GameOutcome::Draw(_) => 1,
    }
}

fn points_for_black(outcome: GameOutcome) -> u32 {
    2 - points_for_white(outcome)
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;

    use super::*;
    use crate::{openings::OpeningId, pairing::IndividualId, self_play::DrawReason};
    use shakmaty::Chess;

    struct RecordingRunner {
        outcomes: std::collections::VecDeque<GameOutcome>,
        calls: Vec<(Genome, Genome, Chess, usize, usize)>,
    }

    impl GameRunner for RecordingRunner {
        type Error = ();
        fn play(
            &mut self,
            white: &Genome,
            black: &Genome,
            opening: &Opening,
            depth: usize,
            max: usize,
        ) -> Result<GameRecord, ()> {
            self.calls.push((
                white.clone(),
                black.clone(),
                opening.position.clone(),
                depth,
                max,
            ));
            let outcome = self.outcomes.pop_front().unwrap();
            Ok(GameRecord {
                outcome,
                moves: vec![],
                position_history: vec![opening.position.clone()],
                final_position: opening.position.clone(),
            })
        }
    }

    #[test]
    fn swaps_colors_reuses_position_and_propagates_depth() {
        let a = Genome::default();
        let mut genes = [1.0; crate::GENE_COUNT];
        genes[0] = 0.5;
        let b = Genome::new(genes).unwrap();
        let opening = Opening {
            id: OpeningId(3),
            seed: 4,
            moves: vec![],
            position: Chess::default(),
        };
        let config = TrainingConfig::new(7, 88, 1, 4..=8, 10).unwrap();
        let mut runner = RecordingRunner {
            outcomes: [GameOutcome::WhiteWin, GameOutcome::WhiteWin].into(),
            calls: vec![],
        };
        let record = play_encounter(
            &mut runner,
            Pairing {
                a: IndividualId(1),
                b: IndividualId(2),
            },
            &a,
            &b,
            &opening,
            &config,
        )
        .unwrap();
        assert_eq!(runner.calls.len(), 2);
        assert_eq!((&runner.calls[0].0, &runner.calls[0].1), (&a, &b));
        assert_eq!((&runner.calls[1].0, &runner.calls[1].1), (&b, &a));
        assert_eq!(runner.calls[0].2, runner.calls[1].2);
        assert!(runner.calls.iter().all(|call| call.3 == 7 && call.4 == 88));
        assert_eq!((record.a_score, record.b_score), (Score(2), Score(2)));
    }

    #[test]
    fn aggregates_win_draw_loss_in_half_points() {
        let opening = Opening {
            id: OpeningId(0),
            seed: 0,
            moves: vec![],
            position: Chess::default(),
        };
        let config = TrainingConfig::new(1, 1, 1, 0..=0, 1).unwrap();
        let genome = Genome::default();
        let mut runner = RecordingRunner {
            outcomes: [
                GameOutcome::WhiteWin,
                GameOutcome::Draw(DrawReason::MaxPlies),
            ]
            .into(),
            calls: vec![],
        };
        let result = play_encounter(
            &mut runner,
            Pairing {
                a: IndividualId(0),
                b: IndividualId(1),
            },
            &genome,
            &genome,
            &opening,
            &config,
        )
        .unwrap();
        assert_eq!(result.a_score.points(), 1.5);
        assert_eq!(result.b_score.points(), 0.5);
    }

    #[test]
    fn every_outcome_pair_conserves_four_half_points() {
        let outcomes = [
            GameOutcome::WhiteWin,
            GameOutcome::BlackWin,
            GameOutcome::Draw(DrawReason::MaxPlies),
        ];
        let opening = Opening {
            id: OpeningId(0),
            seed: 0,
            moves: vec![],
            position: Chess::default(),
        };
        let config = TrainingConfig::new(1, 1, 1, 0..=0, 1).unwrap();
        let genome = Genome::default();
        for first in outcomes {
            for second in outcomes {
                let mut runner = RecordingRunner {
                    outcomes: [first, second].into(),
                    calls: vec![],
                };
                let result = play_encounter(
                    &mut runner,
                    Pairing {
                        a: IndividualId(0),
                        b: IndividualId(1),
                    },
                    &genome,
                    &genome,
                    &opening,
                    &config,
                )
                .unwrap();
                assert_eq!(result.a_score.0 + result.b_score.0, 4);
            }
        }
    }

    #[test]
    fn every_pairing_in_a_round_uses_the_same_opening() {
        let opening = Opening {
            id: OpeningId(7),
            seed: 9,
            moves: vec![],
            position: Chess::default(),
        };
        let round = Round {
            number: 0,
            opening: opening.id,
            pairings: vec![
                Pairing {
                    a: IndividualId(0),
                    b: IndividualId(1),
                },
                Pairing {
                    a: IndividualId(2),
                    b: IndividualId(3),
                },
            ],
        };
        let population = (0..4)
            .map(|id| (IndividualId(id), Genome::default()))
            .collect();
        let config = TrainingConfig::new(3, 20, 1, 0..=0, 1).unwrap();
        let mut runner = RecordingRunner {
            outcomes: [GameOutcome::WhiteWin; 4].into(),
            calls: vec![],
        };

        let records = play_round(&mut runner, &round, &population, &opening, &config).unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(runner.calls.len(), 4);
        assert!(runner.calls.iter().all(|call| call.2 == opening.position));
    }

    #[derive(Clone)]
    struct DrawRunnerFactory {
        calls: Arc<AtomicUsize>,
    }

    struct ParallelDrawRunner {
        calls: Arc<AtomicUsize>,
    }

    impl GameRunnerFactory for DrawRunnerFactory {
        type Runner = ParallelDrawRunner;

        fn create(&self) -> Self::Runner {
            ParallelDrawRunner {
                calls: Arc::clone(&self.calls),
            }
        }
    }

    impl GameRunner for ParallelDrawRunner {
        type Error = &'static str;

        fn play(
            &mut self,
            _white: &Genome,
            _black: &Genome,
            opening: &Opening,
            _search_depth: usize,
            _max_game_plies: usize,
        ) -> Result<GameRecord, Self::Error> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(GameRecord {
                outcome: GameOutcome::Draw(DrawReason::MaxPlies),
                moves: vec![],
                position_history: vec![opening.position.clone()],
                final_position: opening.position.clone(),
            })
        }
    }

    #[test]
    fn parallel_round_preserves_pairing_order_and_executes_every_paired_game_once() {
        let opening = Opening {
            id: OpeningId(7),
            seed: 9,
            moves: vec![],
            position: Chess::default(),
        };
        let round = Round {
            number: 0,
            opening: opening.id,
            pairings: vec![
                Pairing {
                    a: IndividualId(0),
                    b: IndividualId(1),
                },
                Pairing {
                    a: IndividualId(2),
                    b: IndividualId(3),
                },
            ],
        };
        let population = (0..4)
            .map(|id| (IndividualId(id), Genome::default()))
            .collect();
        let config = TrainingConfig::new(3, 20, 1, 0..=0, 1).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut executor = ParallelRoundExecutor::new(
            DrawRunnerFactory {
                calls: Arc::clone(&calls),
            },
            NonZeroUsize::new(4).unwrap(),
        );

        let records = executor
            .play_round(&round, &population, &opening, &config)
            .unwrap();

        assert_eq!(
            records
                .iter()
                .map(|record| record.pairing)
                .collect::<Vec<_>>(),
            round.pairings
        );
        assert_eq!(calls.load(Ordering::SeqCst), 4);
    }

    struct DelayedErrorFactory;
    struct DelayedErrorRunner;

    impl GameRunnerFactory for DelayedErrorFactory {
        type Runner = DelayedErrorRunner;

        fn create(&self) -> Self::Runner {
            DelayedErrorRunner
        }
    }

    impl GameRunner for DelayedErrorRunner {
        type Error = &'static str;

        fn play(
            &mut self,
            white: &Genome,
            _black: &Genome,
            _opening: &Opening,
            _search_depth: usize,
            _max_game_plies: usize,
        ) -> Result<GameRecord, Self::Error> {
            if white.genes()[0] < 0.15 {
                std::thread::sleep(Duration::from_millis(20));
                Err("first pairing")
            } else {
                Err("later pairing")
            }
        }
    }

    #[test]
    fn parallel_round_reports_the_first_logical_error_not_the_fastest_error() {
        let opening = Opening {
            id: OpeningId(0),
            seed: 0,
            moves: vec![],
            position: Chess::default(),
        };
        let round = Round {
            number: 0,
            opening: opening.id,
            pairings: vec![
                Pairing {
                    a: IndividualId(0),
                    b: IndividualId(1),
                },
                Pairing {
                    a: IndividualId(2),
                    b: IndividualId(3),
                },
            ],
        };
        let population = [0.1, 1.0, 0.2, 1.0]
            .into_iter()
            .enumerate()
            .map(|(id, marker)| {
                let mut genes = [1.0; crate::GENE_COUNT];
                genes[0] = marker;
                (IndividualId(id as u64), Genome::new(genes).unwrap())
            })
            .collect();
        let config = TrainingConfig::new(1, 1, 1, 0..=0, 1).unwrap();
        let mut executor =
            ParallelRoundExecutor::new(DelayedErrorFactory, NonZeroUsize::new(2).unwrap());

        assert!(matches!(
            executor.play_round(&round, &population, &opening, &config),
            Err(RoundExecutionError::Game("first pairing"))
        ));
    }

    #[test]
    fn production_runner_completes_a_real_paired_encounter() {
        let opening = Opening {
            id: OpeningId(0),
            seed: 0,
            moves: vec![],
            position: Chess::default(),
        };
        let config = TrainingConfig::new(1, 1, 1, 0..=0, 1).unwrap();
        let genome = Genome::default();
        let mut runner = ProductionGameRunner;

        let result = play_encounter(
            &mut runner,
            Pairing {
                a: IndividualId(0),
                b: IndividualId(1),
            },
            &genome,
            &genome,
            &opening,
            &config,
        )
        .expect("depth-one searches complete");

        assert_eq!(result.first_game.moves.len(), 1);
        assert_eq!(result.second_game.moves.len(), 1);
        assert_eq!(
            result.first_game.outcome,
            GameOutcome::Draw(DrawReason::MaxPlies)
        );
        assert_eq!(
            result.second_game.outcome,
            GameOutcome::Draw(DrawReason::MaxPlies)
        );
        assert_eq!((result.a_score, result.b_score), (Score(2), Score(2)));
    }
}
