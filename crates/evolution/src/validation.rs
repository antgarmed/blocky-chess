//! External, held-out validation of an evolved candidate against the engine
//! reference configuration.

use std::{collections::BTreeSet, error::Error, fmt, ops::RangeInclusive};

use blocky_chess::EvaluationConfig;

use crate::{
    encounter::{ConfiguredGameRunner, ProductionGameRunner},
    genome::Genome,
    openings::{OpeningGenerationError, OpeningId, OpeningPool},
    pairing::Score,
    self_play::GameOutcome,
    training::{TrainingConfig, TrainingConfigError},
};

const DEFAULT_OPENING_COUNT: usize = 20;
const DEFAULT_VALIDATION_SEED: u64 = 0x5641_4c49_4441_5445;
const DEFAULT_MINIMUM_MARGIN_HALF_POINTS: u32 = 1;

/// Hyperparameters for validation. Its seed belongs to a stream independent
/// from training and is recorded in the resulting report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationConfig {
    search_depths: Vec<usize>,
    opening_count: usize,
    max_game_plies: usize,
    master_seed: u64,
    opening_plies: RangeInclusive<usize>,
    max_opening_attempts: usize,
    minimum_margin_half_points: u32,
}

impl ValidationConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        search_depths: Vec<usize>,
        opening_count: usize,
        max_game_plies: usize,
        master_seed: u64,
        opening_plies: RangeInclusive<usize>,
        max_opening_attempts: usize,
        minimum_margin_half_points: u32,
    ) -> Result<Self, ValidationConfigError> {
        if search_depths.is_empty() {
            return Err(ValidationConfigError::NoSearchDepths);
        }
        if search_depths.contains(&0) {
            return Err(ValidationConfigError::ZeroSearchDepth);
        }
        if search_depths.iter().copied().collect::<BTreeSet<_>>().len() != search_depths.len() {
            return Err(ValidationConfigError::DuplicateSearchDepth);
        }
        if opening_count == 0 {
            return Err(ValidationConfigError::ZeroOpenings);
        }
        TrainingConfig::new(
            search_depths[0],
            max_game_plies,
            master_seed,
            opening_plies.clone(),
            max_opening_attempts,
        )
        .map_err(ValidationConfigError::Training)?;
        Ok(Self {
            search_depths,
            opening_count,
            max_game_plies,
            master_seed,
            opening_plies,
            max_opening_attempts,
            minimum_margin_half_points,
        })
    }

    pub fn search_depths(&self) -> &[usize] {
        &self.search_depths
    }
    pub const fn opening_count(&self) -> usize {
        self.opening_count
    }
    pub const fn max_game_plies(&self) -> usize {
        self.max_game_plies
    }
    pub const fn master_seed(&self) -> u64 {
        self.master_seed
    }
    pub const fn opening_plies(&self) -> &RangeInclusive<usize> {
        &self.opening_plies
    }
    pub const fn max_opening_attempts(&self) -> usize {
        self.max_opening_attempts
    }
    pub const fn minimum_margin_half_points(&self) -> u32 {
        self.minimum_margin_half_points
    }

    fn training_at_depth(&self, depth: usize) -> TrainingConfig {
        TrainingConfig::new(
            depth,
            self.max_game_plies,
            self.master_seed,
            self.opening_plies.clone(),
            self.max_opening_attempts,
        )
        .expect("validation configuration was checked at construction")
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self::new(
            vec![4, 6],
            DEFAULT_OPENING_COUNT,
            200,
            DEFAULT_VALIDATION_SEED,
            4..=10,
            100,
            DEFAULT_MINIMUM_MARGIN_HALF_POINTS,
        )
        .expect("built-in validation defaults are valid")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationConfigError {
    NoSearchDepths,
    ZeroSearchDepth,
    DuplicateSearchDepth,
    ZeroOpenings,
    Training(TrainingConfigError),
}

impl fmt::Display for ValidationConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl Error for ValidationConfigError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpeningValidationResult {
    pub opening: OpeningId,
    pub opening_seed: u64,
    pub candidate_score: Score,
    pub reference_score: Score,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepthValidationResult {
    pub search_depth: usize,
    pub candidate_score: Score,
    pub reference_score: Score,
    pub accepted: bool,
    pub openings: Vec<OpeningValidationResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationReport {
    pub config: ValidationConfig,
    pub by_depth: Vec<DepthValidationResult>,
    pub candidate_score: Score,
    pub reference_score: Score,
    pub accepted: bool,
}

/// Runs the external benchmark against the literal engine default, which is
/// deliberately created here and nowhere in the evolutionary loop.
pub struct ChampionValidator<R> {
    config: ValidationConfig,
    runner: R,
}

impl<R> ChampionValidator<R> {
    pub fn new(config: ValidationConfig, runner: R) -> Self {
        Self { config, runner }
    }

    pub const fn config(&self) -> &ValidationConfig {
        &self.config
    }
}

impl ChampionValidator<ProductionGameRunner> {
    pub fn production(config: ValidationConfig) -> Self {
        Self::new(config, ProductionGameRunner)
    }
}

impl<R: ConfiguredGameRunner> ChampionValidator<R> {
    pub fn validate(
        &mut self,
        candidate: &Genome,
    ) -> Result<ValidationReport, ValidationError<R::Error>> {
        let opening_config = self.config.training_at_depth(self.config.search_depths[0]);
        let pool = OpeningPool::generate(self.config.opening_count, &opening_config)
            .map_err(ValidationError::Opening)?;
        let candidate_config = candidate.to_evaluation_config();
        let reference = EvaluationConfig::default();
        let mut by_depth = Vec::with_capacity(self.config.search_depths.len());

        for &depth in &self.config.search_depths {
            let mut candidate_score = Score(0);
            let mut reference_score = Score(0);
            let mut openings = Vec::with_capacity(pool.openings().len());
            for opening in pool.openings() {
                let first = self
                    .runner
                    .play_configured(
                        candidate_config,
                        reference,
                        opening,
                        depth,
                        self.config.max_game_plies,
                    )
                    .map_err(ValidationError::Game)?;
                let second = self
                    .runner
                    .play_configured(
                        reference,
                        candidate_config,
                        opening,
                        depth,
                        self.config.max_game_plies,
                    )
                    .map_err(ValidationError::Game)?;
                let opening_candidate_score =
                    Score(points_for_white(first.outcome) + points_for_black(second.outcome));
                let opening_reference_score = Score(4 - opening_candidate_score.0);
                candidate_score.0 += opening_candidate_score.0;
                reference_score.0 += opening_reference_score.0;
                openings.push(OpeningValidationResult {
                    opening: opening.id,
                    opening_seed: opening.seed,
                    candidate_score: opening_candidate_score,
                    reference_score: opening_reference_score,
                });
            }
            by_depth.push(DepthValidationResult {
                search_depth: depth,
                candidate_score,
                reference_score,
                accepted: candidate_score.0
                    >= reference_score
                        .0
                        .saturating_add(self.config.minimum_margin_half_points),
                openings,
            });
        }

        let candidate_score = Score(by_depth.iter().map(|result| result.candidate_score.0).sum());
        let reference_score = Score(by_depth.iter().map(|result| result.reference_score.0).sum());
        let accepted = by_depth.iter().all(|result| result.accepted);
        Ok(ValidationReport {
            config: self.config.clone(),
            by_depth,
            candidate_score,
            reference_score,
            accepted,
        })
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

#[derive(Debug)]
pub enum ValidationError<E> {
    Opening(OpeningGenerationError),
    Game(E),
}

impl<E: fmt::Display> fmt::Display for ValidationError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Opening(source) => {
                write!(formatter, "held-out opening generation failed: {source}")
            }
            Self::Game(source) => write!(formatter, "validation game failed: {source}"),
        }
    }
}

impl<E: Error + 'static> Error for ValidationError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Opening(source) => Some(source),
            Self::Game(source) => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use shakmaty::Chess;

    use super::*;
    use crate::self_play::{DrawReason, GameOutcome, GameRecord};

    #[derive(Default)]
    struct RecordingRunner {
        outcomes: VecDeque<Result<GameOutcome, &'static str>>,
        calls: Vec<(EvaluationConfig, EvaluationConfig, OpeningId, u64, usize)>,
    }

    impl ConfiguredGameRunner for RecordingRunner {
        type Error = &'static str;

        fn play_configured(
            &mut self,
            white: EvaluationConfig,
            black: EvaluationConfig,
            opening: &crate::openings::Opening,
            search_depth: usize,
            _max_game_plies: usize,
        ) -> Result<GameRecord, Self::Error> {
            self.calls
                .push((white, black, opening.id, opening.seed, search_depth));
            let outcome = self
                .outcomes
                .pop_front()
                .unwrap_or(Ok(GameOutcome::Draw(DrawReason::MaxPlies)))?;
            Ok(GameRecord {
                outcome,
                moves: vec![],
                position_history: vec![opening.position.clone()],
                final_position: Chess::default(),
            })
        }
    }

    fn config(depths: Vec<usize>, openings: usize, margin: u32) -> ValidationConfig {
        ValidationConfig::new(depths, openings, 20, 99, 2..=2, 100, margin).unwrap()
    }

    fn candidate() -> Genome {
        Genome::new([1.0; crate::GENE_COUNT]).unwrap()
    }

    #[test]
    fn rejects_invalid_hyperparameters() {
        assert_eq!(
            ValidationConfig::new(vec![], 1, 10, 2, 0..=0, 1, 1),
            Err(ValidationConfigError::NoSearchDepths)
        );
        assert_eq!(
            ValidationConfig::new(vec![0], 1, 10, 2, 0..=0, 1, 1),
            Err(ValidationConfigError::ZeroSearchDepth)
        );
        assert_eq!(
            ValidationConfig::new(vec![2, 2], 1, 10, 2, 0..=0, 1, 1),
            Err(ValidationConfigError::DuplicateSearchDepth)
        );
        assert_eq!(
            ValidationConfig::new(vec![2], 0, 10, 2, 0..=0, 1, 1),
            Err(ValidationConfigError::ZeroOpenings)
        );
    }

    #[test]
    fn defaults_use_held_out_multi_depth_validation_and_a_strict_majority() {
        let config = ValidationConfig::default();

        assert_eq!(config.search_depths(), &[4, 6]);
        assert_eq!(config.opening_count(), 20);
        assert_eq!(config.minimum_margin_half_points(), 1);
        assert_ne!(
            config.master_seed(),
            TrainingConfig::default().master_seed()
        );
    }

    #[test]
    fn uses_same_held_out_openings_at_every_depth_and_swaps_colors() {
        let candidate = candidate();
        let mut validator =
            ChampionValidator::new(config(vec![3, 7], 2, 1), RecordingRunner::default());

        let report = validator.validate(&candidate).unwrap();

        assert_eq!(report.by_depth.len(), 2);
        assert_eq!(
            report.by_depth[0]
                .openings
                .iter()
                .map(|opening| (opening.opening, opening.opening_seed))
                .collect::<Vec<_>>(),
            report.by_depth[1]
                .openings
                .iter()
                .map(|opening| (opening.opening, opening.opening_seed))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            validator
                .runner
                .calls
                .iter()
                .map(|call| call.4)
                .collect::<Vec<_>>(),
            vec![3, 3, 3, 3, 7, 7, 7, 7]
        );
        for games in validator.runner.calls.chunks_exact(2) {
            assert_eq!(games[0].0, candidate.to_evaluation_config());
            assert_eq!(games[0].1, EvaluationConfig::default());
            assert_eq!(games[1].0, EvaluationConfig::default());
            assert_eq!(games[1].1, candidate.to_evaluation_config());
            assert_eq!((games[0].2, games[0].3), (games[1].2, games[1].3));
        }
    }

    #[test]
    fn aggregates_half_points_and_applies_configured_margin() {
        let outcomes = [
            GameOutcome::WhiteWin,
            GameOutcome::BlackWin,
            GameOutcome::Draw(DrawReason::MaxPlies),
            GameOutcome::WhiteWin,
        ]
        .into_iter()
        .map(Ok)
        .collect();
        let runner = RecordingRunner {
            outcomes,
            ..RecordingRunner::default()
        };
        let mut validator = ChampionValidator::new(config(vec![2], 2, 3), runner);

        let report = validator.validate(&candidate()).unwrap();

        assert_eq!(report.candidate_score, Score(5));
        assert_eq!(report.reference_score, Score(3));
        assert!(!report.accepted);
        assert!(!report.by_depth[0].accepted);
        assert_eq!(report.by_depth[0].openings[0].candidate_score, Score(4));
        assert_eq!(report.by_depth[0].openings[1].candidate_score, Score(1));
    }

    #[test]
    fn requires_the_margin_at_every_depth_instead_of_only_in_aggregate() {
        let outcomes = [
            GameOutcome::WhiteWin,
            GameOutcome::BlackWin,
            GameOutcome::WhiteWin,
            GameOutcome::BlackWin,
            GameOutcome::BlackWin,
            GameOutcome::WhiteWin,
            GameOutcome::BlackWin,
            GameOutcome::WhiteWin,
        ]
        .into_iter()
        .map(Ok)
        .collect();
        let runner = RecordingRunner {
            outcomes,
            ..RecordingRunner::default()
        };
        let configuration = config(vec![2, 5], 2, 1);
        let mut validator = ChampionValidator::new(configuration.clone(), runner);

        let report = validator.validate(&candidate()).unwrap();

        assert_eq!(report.config, configuration);
        assert_eq!(report.candidate_score, Score(8));
        assert_eq!(report.reference_score, Score(8));
        assert!(report.by_depth[0].accepted);
        assert!(!report.by_depth[1].accepted);
        assert!(!report.accepted);
    }

    #[test]
    fn is_deterministic_and_propagates_game_errors() {
        let mut first = ChampionValidator::new(config(vec![2], 2, 0), RecordingRunner::default());
        let mut second = ChampionValidator::new(config(vec![2], 2, 0), RecordingRunner::default());
        assert_eq!(
            first.validate(&candidate()).unwrap(),
            second.validate(&candidate()).unwrap()
        );

        let runner = RecordingRunner {
            outcomes: [Err("boom")].into(),
            ..RecordingRunner::default()
        };
        let mut failing = ChampionValidator::new(config(vec![2], 1, 1), runner);
        assert!(matches!(
            failing.validate(&candidate()),
            Err(ValidationError::Game("boom"))
        ));
    }
}
