//! Exploratory controls for checkpoint candidates.

use std::{error::Error, fmt, num::NonZeroUsize, ops::RangeInclusive};

use serde::Serialize;

use crate::{
    genome::{Genome, GENE_COUNT},
    openings::{Opening, OpeningGenerationError, OpeningPool},
    rng::{derive_seed, RandomSource, StableRng},
    self_play::{
        GameError, GameOutcome, RandomLegalMoveSelector, SearchMoveSelector,
        SearchMoveSelectorError, SelfPlayGame,
    },
    telemetry::{GameObservation, GameStatistics},
    training::TrainingConfig,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlOpponent {
    RandomLegal,
    RandomGenome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkConfig {
    pub search_depth: usize,
    pub opening_count: usize,
    pub max_game_plies: usize,
    pub benchmark_seed: u64,
    pub opponent_seed: u64,
    pub random_genome_count: usize,
    pub opening_plies: RangeInclusive<usize>,
    pub max_opening_attempts: usize,
}

impl BenchmarkConfig {
    pub fn validate(&self) -> Result<(), BenchmarkError> {
        let checks = [
            (self.search_depth > 0, "benchmark depth must be positive"),
            (
                self.opening_count > 0,
                "benchmark openings must be positive",
            ),
            (
                self.max_game_plies > 0,
                "benchmark max game plies must be positive",
            ),
            (
                self.random_genome_count > 0,
                "random genome count must be positive",
            ),
            (
                self.opening_plies.start() <= self.opening_plies.end(),
                "benchmark opening minimum must not exceed maximum",
            ),
            (
                self.max_opening_attempts > 0,
                "benchmark opening attempts must be positive",
            ),
        ];
        checks
            .into_iter()
            .find_map(|(valid, message)| (!valid).then_some(BenchmarkError::InvalidConfig(message)))
            .map_or(Ok(()), Err)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BenchmarkReport {
    pub search_depth: usize,
    pub opening_count: usize,
    pub max_game_plies: usize,
    pub benchmark_seed: u64,
    pub opponent_seed: u64,
    pub opening_min_plies: usize,
    pub opening_max_plies: usize,
    pub max_opening_attempts: usize,
    pub random_genome_count: usize,
    pub random_genomes: Vec<[f64; GENE_COUNT]>,
    pub controls: Vec<ControlResult>,
    pub random_genome_ensemble: EnsembleResult,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EnsembleResult {
    pub opponent_count: usize,
    pub candidate_score_half_points: u32,
    pub opponents_score_half_points: u32,
    pub statistics: SerializableStatistics,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ControlResult {
    pub opponent: ControlOpponent,
    pub opponent_index: Option<usize>,
    pub candidate_score_half_points: u32,
    pub opponent_score_half_points: u32,
    pub statistics: SerializableStatistics,
    pub openings: Vec<OpeningResult>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OpeningResult {
    pub opening_id: u64,
    pub opening_seed: u64,
    pub candidate_score_half_points: u32,
    pub opponent_score_half_points: u32,
    pub games: [SerializableObservation; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SerializableObservation {
    pub outcome: &'static str,
    pub draw_reason: Option<&'static str>,
    pub plies: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SerializableStatistics {
    pub games: usize,
    pub white_wins: usize,
    pub black_wins: usize,
    pub draws: usize,
    pub stalemates: usize,
    pub insufficient_material: usize,
    pub threefold_repetitions: usize,
    pub fifty_move_rule: usize,
    pub max_plies_draws: usize,
    pub total_plies: usize,
    pub minimum_plies: usize,
    pub median_plies: usize,
    pub p95_plies: usize,
    pub maximum_plies: usize,
    pub mean_plies: f64,
}

impl From<GameObservation> for SerializableObservation {
    fn from(value: GameObservation) -> Self {
        use crate::self_play::DrawReason::*;
        let (outcome, draw_reason) = match value.outcome {
            GameOutcome::WhiteWin => ("white_win", None),
            GameOutcome::BlackWin => ("black_win", None),
            GameOutcome::Draw(reason) => (
                "draw",
                Some(match reason {
                    Stalemate => "stalemate",
                    InsufficientMaterial => "insufficient_material",
                    ThreefoldRepetition => "threefold_repetition",
                    FiftyMoveRule => "fifty_move_rule",
                    MaxPlies => "max_plies",
                }),
            ),
        };
        Self {
            outcome,
            draw_reason,
            plies: value.plies,
        }
    }
}

impl From<GameStatistics> for SerializableStatistics {
    fn from(value: GameStatistics) -> Self {
        Self {
            games: value.games,
            white_wins: value.white_wins,
            black_wins: value.black_wins,
            draws: value.draws,
            stalemates: value.stalemates,
            insufficient_material: value.insufficient_material,
            threefold_repetitions: value.threefold_repetitions,
            fifty_move_rule: value.fifty_move_rule,
            max_plies_draws: value.max_plies_draws,
            total_plies: value.total_plies,
            minimum_plies: value.minimum_plies,
            median_plies: value.median_plies,
            p95_plies: value.p95_plies,
            maximum_plies: value.maximum_plies,
            mean_plies: value.mean_plies(),
        }
    }
}

#[derive(Debug)]
pub enum BenchmarkError {
    InvalidConfig(&'static str),
    Opening(OpeningGenerationError),
    Selector(SearchMoveSelectorError),
    Game(GameError),
    WorkerPanic,
}

impl fmt::Display for BenchmarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for BenchmarkError {}
impl From<SearchMoveSelectorError> for BenchmarkError {
    fn from(value: SearchMoveSelectorError) -> Self {
        Self::Selector(value)
    }
}
impl From<GameError> for BenchmarkError {
    fn from(value: GameError) -> Self {
        Self::Game(value)
    }
}

pub fn run_benchmark(
    candidate: &Genome,
    config: &BenchmarkConfig,
    workers: NonZeroUsize,
) -> Result<BenchmarkReport, BenchmarkError> {
    run_benchmark_with_observer(candidate, config, workers, &mut |_| {})
}

pub fn run_benchmark_with_observer(
    candidate: &Genome,
    config: &BenchmarkConfig,
    workers: NonZeroUsize,
    observer: &mut dyn FnMut(&ControlResult),
) -> Result<BenchmarkReport, BenchmarkError> {
    config.validate()?;
    let opening_config = TrainingConfig::new(
        config.search_depth,
        config.max_game_plies,
        config.benchmark_seed,
        config.opening_plies.clone(),
        config.max_opening_attempts,
    )
    .map_err(|_| BenchmarkError::InvalidConfig("invalid opening configuration"))?;
    let pool = OpeningPool::generate(config.opening_count, &opening_config)
        .map_err(BenchmarkError::Opening)?;
    let random_genomes = generate_random_genomes(config.opponent_seed, config.random_genome_count);
    let mut controls = vec![run_control(
        candidate,
        None,
        ControlOpponent::RandomLegal,
        pool.openings(),
        config,
        workers,
    )?];
    observer(&controls[0]);
    for (index, genome) in random_genomes.iter().enumerate() {
        let result = run_control(
            candidate,
            Some(genome),
            ControlOpponent::RandomGenome,
            pool.openings(),
            config,
            workers,
        )?
        .with_index(index);
        observer(&result);
        controls.push(result);
    }
    let ensemble_controls = &controls[1..];
    let ensemble_statistics = GameStatistics::from_observations(
        ensemble_controls
            .iter()
            .flat_map(|control| &control.openings)
            .flat_map(|opening| opening.games)
            .map(observation_from_serializable),
    );
    let random_genome_ensemble = EnsembleResult {
        opponent_count: ensemble_controls.len(),
        candidate_score_half_points: ensemble_controls
            .iter()
            .map(|control| control.candidate_score_half_points)
            .sum(),
        opponents_score_half_points: ensemble_controls
            .iter()
            .map(|control| control.opponent_score_half_points)
            .sum(),
        statistics: ensemble_statistics.into(),
    };
    Ok(BenchmarkReport {
        search_depth: config.search_depth,
        opening_count: config.opening_count,
        max_game_plies: config.max_game_plies,
        benchmark_seed: config.benchmark_seed,
        opponent_seed: config.opponent_seed,
        opening_min_plies: *config.opening_plies.start(),
        opening_max_plies: *config.opening_plies.end(),
        max_opening_attempts: config.max_opening_attempts,
        random_genome_count: config.random_genome_count,
        random_genomes: random_genomes
            .iter()
            .map(|genome| *genome.genes())
            .collect(),
        controls,
        random_genome_ensemble,
    })
}

fn observation_from_serializable(game: SerializableObservation) -> GameObservation {
    use crate::self_play::DrawReason::*;
    GameObservation {
        outcome: match (game.outcome, game.draw_reason) {
            ("white_win", _) => GameOutcome::WhiteWin,
            ("black_win", _) => GameOutcome::BlackWin,
            ("draw", Some("stalemate")) => GameOutcome::Draw(Stalemate),
            ("draw", Some("insufficient_material")) => GameOutcome::Draw(InsufficientMaterial),
            ("draw", Some("threefold_repetition")) => GameOutcome::Draw(ThreefoldRepetition),
            ("draw", Some("fifty_move_rule")) => GameOutcome::Draw(FiftyMoveRule),
            _ => GameOutcome::Draw(MaxPlies),
        },
        plies: game.plies,
    }
}

impl ControlResult {
    fn with_index(mut self, index: usize) -> Self {
        self.opponent_index = Some(index);
        self
    }
}

fn generate_random_genomes(seed: u64, count: usize) -> Vec<Genome> {
    let mut rng = StableRng::new(seed);
    (0..count)
        .map(|_| {
            let mut genes = [0.0; GENE_COUNT];
            for gene in &mut genes {
                *gene = rng.unit_f64();
            }
            Genome::new(genes).expect("stable RNG produces a valid genome")
        })
        .collect()
}

fn run_control(
    candidate: &Genome,
    opponent: Option<&Genome>,
    kind: ControlOpponent,
    openings: &[Opening],
    config: &BenchmarkConfig,
    workers: NonZeroUsize,
) -> Result<ControlResult, BenchmarkError> {
    let worker_count = workers.get().min(openings.len());
    let chunks = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for worker in 0..worker_count {
            handles.push(scope.spawn(move || {
                openings
                    .iter()
                    .enumerate()
                    .skip(worker)
                    .step_by(worker_count)
                    .map(|(index, opening)| {
                        (index, play_pair(candidate, opponent, kind, opening, config))
                    })
                    .collect::<Vec<_>>()
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join())
            .collect::<Vec<_>>()
    });
    let mut ordered = vec![None; openings.len()];
    for chunk in chunks {
        for (index, result) in chunk.map_err(|_| BenchmarkError::WorkerPanic)? {
            ordered[index] = Some(result?);
        }
    }
    let openings = ordered.into_iter().map(Option::unwrap).collect::<Vec<_>>();
    let candidate_score_half_points = openings
        .iter()
        .map(|opening| opening.candidate_score_half_points)
        .sum();
    let observations = openings
        .iter()
        .flat_map(|opening| opening.games)
        .map(observation_from_serializable);
    Ok(ControlResult {
        opponent: kind,
        opponent_index: None,
        candidate_score_half_points,
        opponent_score_half_points: (openings.len() as u32 * 4) - candidate_score_half_points,
        statistics: GameStatistics::from_observations(observations).into(),
        openings,
    })
}

fn play_pair(
    candidate: &Genome,
    opponent: Option<&Genome>,
    kind: ControlOpponent,
    opening: &Opening,
    config: &BenchmarkConfig,
) -> Result<OpeningResult, BenchmarkError> {
    let candidate_config = candidate.to_evaluation_config();
    let seed_a = derive_seed(config.benchmark_seed, opening.id.0, 0);
    let seed_b = derive_seed(config.benchmark_seed, opening.id.0, 1);
    let first = match kind {
        ControlOpponent::RandomLegal => SelfPlayGame::from_position(
            opening.position.clone(),
            SearchMoveSelector::alpha_beta(candidate_config, config.search_depth)?,
            RandomLegalMoveSelector::new(seed_a),
            config.max_game_plies,
        )
        .play()?,
        ControlOpponent::RandomGenome => SelfPlayGame::from_position(
            opening.position.clone(),
            SearchMoveSelector::alpha_beta(candidate_config, config.search_depth)?,
            SearchMoveSelector::alpha_beta(
                opponent
                    .expect("random-genome control has a genome")
                    .to_evaluation_config(),
                config.search_depth,
            )?,
            config.max_game_plies,
        )
        .play()?,
    };
    let second = match kind {
        ControlOpponent::RandomLegal => SelfPlayGame::from_position(
            opening.position.clone(),
            RandomLegalMoveSelector::new(seed_b),
            SearchMoveSelector::alpha_beta(candidate_config, config.search_depth)?,
            config.max_game_plies,
        )
        .play()?,
        ControlOpponent::RandomGenome => SelfPlayGame::from_position(
            opening.position.clone(),
            SearchMoveSelector::alpha_beta(
                opponent
                    .expect("random-genome control has a genome")
                    .to_evaluation_config(),
                config.search_depth,
            )?,
            SearchMoveSelector::alpha_beta(candidate_config, config.search_depth)?,
            config.max_game_plies,
        )
        .play()?,
    };
    let score = points_for_white(first.outcome) + points_for_black(second.outcome);
    Ok(OpeningResult {
        opening_id: opening.id.0,
        opening_seed: opening.seed,
        candidate_score_half_points: score,
        opponent_score_half_points: 4 - score,
        games: [
            GameObservation::from(&first).into(),
            GameObservation::from(&second).into(),
        ],
    })
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
    use super::*;
    use crate::self_play::MoveSelector;
    use shakmaty::{Chess, Position};

    #[test]
    fn random_legal_is_reproducible_and_always_returns_a_legal_move() {
        let position = Chess::default();
        let mut first = RandomLegalMoveSelector::new(42);
        let mut second = RandomLegalMoveSelector::new(42);
        for _ in 0..10 {
            let a = first.select_move(&position).unwrap().unwrap();
            let b = second.select_move(&position).unwrap().unwrap();
            assert_eq!(a, b);
            assert!(position.is_legal(a));
        }
    }

    #[test]
    fn generated_ensemble_is_reproducible_and_not_identical() {
        let a = generate_random_genomes(7, 3);
        let b = generate_random_genomes(7, 3);
        assert_eq!(a, b);
        assert_ne!(a[0], a[1]);
    }

    #[test]
    fn production_benchmark_is_identical_for_one_and_many_workers() {
        let config = BenchmarkConfig {
            search_depth: 1,
            opening_count: 3,
            max_game_plies: 1,
            benchmark_seed: 101,
            opponent_seed: 102,
            random_genome_count: 2,
            opening_plies: 2..=2,
            max_opening_attempts: 100,
        };
        let one =
            run_benchmark(&Genome::default(), &config, NonZeroUsize::new(1).unwrap()).unwrap();
        let many =
            run_benchmark(&Genome::default(), &config, NonZeroUsize::new(3).unwrap()).unwrap();
        assert_eq!(one, many);
    }

    #[test]
    fn progress_observer_reports_each_control_without_changing_report() {
        let config = BenchmarkConfig {
            search_depth: 1,
            opening_count: 1,
            max_game_plies: 1,
            benchmark_seed: 201,
            opponent_seed: 202,
            random_genome_count: 2,
            opening_plies: 0..=0,
            max_opening_attempts: 1,
        };
        let expected =
            run_benchmark(&Genome::default(), &config, NonZeroUsize::new(1).unwrap()).unwrap();
        let mut observed = Vec::new();
        let actual = run_benchmark_with_observer(
            &Genome::default(),
            &config,
            NonZeroUsize::new(1).unwrap(),
            &mut |control| observed.push(control.opponent),
        )
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(
            observed,
            vec![
                ControlOpponent::RandomLegal,
                ControlOpponent::RandomGenome,
                ControlOpponent::RandomGenome
            ]
        );
    }

    #[test]
    fn rejects_every_invalid_benchmark_dimension() {
        let reversed_start = 2;
        let reversed_end = 1;
        let valid = BenchmarkConfig {
            search_depth: 1,
            opening_count: 1,
            max_game_plies: 1,
            benchmark_seed: 1,
            opponent_seed: 2,
            random_genome_count: 1,
            opening_plies: 0..=0,
            max_opening_attempts: 1,
        };
        for invalid in [
            BenchmarkConfig {
                search_depth: 0,
                ..valid.clone()
            },
            BenchmarkConfig {
                opening_count: 0,
                ..valid.clone()
            },
            BenchmarkConfig {
                max_game_plies: 0,
                ..valid.clone()
            },
            BenchmarkConfig {
                random_genome_count: 0,
                ..valid.clone()
            },
            BenchmarkConfig {
                opening_plies: reversed_start..=reversed_end,
                ..valid.clone()
            },
            BenchmarkConfig {
                max_opening_attempts: 0,
                ..valid.clone()
            },
        ] {
            assert!(matches!(
                invalid.validate(),
                Err(BenchmarkError::InvalidConfig(_))
            ));
        }
    }
}
