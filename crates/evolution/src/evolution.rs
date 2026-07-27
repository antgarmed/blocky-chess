//! Deterministic generational evolution driven exclusively by contemporary
//! population self-play.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    num::NonZeroUsize,
};

use crate::{
    encounter::{
        AuxiliaryRoundExecutor, ParallelRoundExecutor, RoundExecutionError, RoundExecutor,
        SequentialRoundExecutor,
    },
    genome::{Genome, GenomeError, GENE_COUNT},
    historical::{phenotype_fingerprint, HistoricalArchive, HistoricalAudit, HistoricalConfig},
    openings::{OpeningGenerationError, OpeningPool},
    pairing::{IndividualId, PairingError, Score, Standing, SwissScheduler},
    progress::{NoopProgressObserver, ProgressEvent, ProgressObserver},
    rng::{derive_seed, RandomSource, StableRng},
    telemetry::GameStatistics,
    training::TrainingConfig,
};

const DEFAULT_GENERATIONS: usize = 100;
const DEFAULT_POPULATION_SIZE: usize = 32;
const DEFAULT_SWISS_ROUNDS: usize = 5;
const DEFAULT_ELITE_COUNT: usize = 2;
const DEFAULT_PARENT_CANDIDATE_COUNT: usize = 3;
const DEFAULT_GENE_MUTATION_PROBABILITY: f64 = 0.15;
const DEFAULT_STRONG_MUTATION_PROBABILITY: f64 = 0.02;
const DEFAULT_MUTATION_STEP: f64 = 0.10;
const DEFAULT_STRONG_MUTATION_STEP: f64 = 0.50;
const OFFSPRING_DUPLICATE_RETRIES: usize = 8;
const ANCHOR_SEED_DOMAIN: u64 = 0x414e_4348_4f52;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DefaultAnchorConfig {
    weight_percent: u8,
    opening_pairs: usize,
}

impl DefaultAnchorConfig {
    pub fn new(weight_percent: u8, opening_pairs: usize) -> Result<Self, EvolutionConfigError> {
        if weight_percent > 100 {
            return Err(EvolutionConfigError::AnchorWeightOutOfRange(weight_percent));
        }
        if (weight_percent == 0) != (opening_pairs == 0) {
            return Err(EvolutionConfigError::InconsistentAnchorConfiguration {
                weight_percent,
                opening_pairs,
            });
        }
        Ok(Self {
            weight_percent,
            opening_pairs,
        })
    }

    pub const fn weight_percent(self) -> u8 {
        self.weight_percent
    }

    pub const fn opening_pairs(self) -> usize {
        self.opening_pairs
    }

    pub const fn enabled(self) -> bool {
        self.weight_percent > 0
    }
}

/// All hyperparameters required by an in-memory evolutionary run.
#[derive(Clone, Debug, PartialEq)]
pub struct EvolutionConfig {
    training: TrainingConfig,
    generations: usize,
    population_size: usize,
    swiss_rounds: usize,
    elite_count: usize,
    parent_candidate_count: usize,
    gene_mutation_probability: f64,
    strong_mutation_probability: f64,
    mutation_step: f64,
    strong_mutation_step: f64,
    default_anchor: DefaultAnchorConfig,
    historical: HistoricalConfig,
}

impl EvolutionConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        training: TrainingConfig,
        generations: usize,
        population_size: usize,
        swiss_rounds: usize,
        elite_count: usize,
        parent_candidate_count: usize,
        gene_mutation_probability: f64,
        strong_mutation_probability: f64,
        mutation_step: f64,
        strong_mutation_step: f64,
    ) -> Result<Self, EvolutionConfigError> {
        if generations == 0 {
            return Err(EvolutionConfigError::ZeroGenerations);
        }
        if population_size < 2 {
            return Err(EvolutionConfigError::PopulationTooSmall(population_size));
        }
        if !population_size.is_multiple_of(2) {
            return Err(EvolutionConfigError::OddPopulation(population_size));
        }
        if swiss_rounds == 0 {
            return Err(EvolutionConfigError::ZeroSwissRounds);
        }
        if swiss_rounds >= population_size {
            return Err(EvolutionConfigError::TooManySwissRounds {
                rounds: swiss_rounds,
                population: population_size,
            });
        }
        if elite_count >= population_size {
            return Err(EvolutionConfigError::TooManyElites {
                elites: elite_count,
                population: population_size,
            });
        }
        if parent_candidate_count == 0 || parent_candidate_count >= population_size {
            return Err(EvolutionConfigError::InvalidParentCandidateCount {
                candidates: parent_candidate_count,
                population: population_size,
            });
        }
        validate_probability("gene mutation", gene_mutation_probability)?;
        validate_probability("strong mutation", strong_mutation_probability)?;
        validate_step("mutation", mutation_step)?;
        validate_step("strong mutation", strong_mutation_step)?;

        Ok(Self {
            training,
            generations,
            population_size,
            swiss_rounds,
            elite_count,
            parent_candidate_count,
            gene_mutation_probability,
            strong_mutation_probability,
            mutation_step,
            strong_mutation_step,
            default_anchor: DefaultAnchorConfig::default(),
            historical: HistoricalConfig::default(),
        })
    }

    pub fn with_default_anchor(
        mut self,
        default_anchor: DefaultAnchorConfig,
    ) -> Result<Self, EvolutionConfigError> {
        self.default_anchor =
            DefaultAnchorConfig::new(default_anchor.weight_percent, default_anchor.opening_pairs)?;
        if self.default_anchor.enabled()
            && 400_u128 * self.swiss_rounds as u128 * self.default_anchor.opening_pairs as u128
                > u128::from(u32::MAX)
        {
            return Err(EvolutionConfigError::AnchorScoreOverflow {
                swiss_rounds: self.swiss_rounds,
                opening_pairs: self.default_anchor.opening_pairs,
            });
        }
        Ok(self)
    }

    pub const fn training(&self) -> &TrainingConfig {
        &self.training
    }
    pub const fn generations(&self) -> usize {
        self.generations
    }
    pub const fn population_size(&self) -> usize {
        self.population_size
    }
    pub const fn swiss_rounds(&self) -> usize {
        self.swiss_rounds
    }
    pub const fn elite_count(&self) -> usize {
        self.elite_count
    }
    pub const fn parent_candidate_count(&self) -> usize {
        self.parent_candidate_count
    }
    pub const fn gene_mutation_probability(&self) -> f64 {
        self.gene_mutation_probability
    }
    pub const fn strong_mutation_probability(&self) -> f64 {
        self.strong_mutation_probability
    }
    pub const fn mutation_step(&self) -> f64 {
        self.mutation_step
    }
    pub const fn strong_mutation_step(&self) -> f64 {
        self.strong_mutation_step
    }
    pub const fn default_anchor(&self) -> DefaultAnchorConfig {
        self.default_anchor
    }
    pub fn with_historical(
        mut self,
        historical: HistoricalConfig,
    ) -> Result<Self, EvolutionConfigError> {
        if historical.enabled() && self.default_anchor.enabled() {
            return Err(EvolutionConfigError::ConflictingTrainingObjectives);
        }
        self.historical = historical;
        Ok(self)
    }
    pub const fn historical(&self) -> HistoricalConfig {
        self.historical
    }
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self::new(
            TrainingConfig::default(),
            DEFAULT_GENERATIONS,
            DEFAULT_POPULATION_SIZE,
            DEFAULT_SWISS_ROUNDS,
            DEFAULT_ELITE_COUNT,
            DEFAULT_PARENT_CANDIDATE_COUNT,
            DEFAULT_GENE_MUTATION_PROBABILITY,
            DEFAULT_STRONG_MUTATION_PROBABILITY,
            DEFAULT_MUTATION_STEP,
            DEFAULT_STRONG_MUTATION_STEP,
        )
        .expect("built-in evolution defaults are valid")
    }
}

fn validate_probability(name: &'static str, value: f64) -> Result<(), EvolutionConfigError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(EvolutionConfigError::InvalidProbability { name, value });
    }
    Ok(())
}

fn validate_step(name: &'static str, value: f64) -> Result<(), EvolutionConfigError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(EvolutionConfigError::InvalidMutationStep { name, value });
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub enum EvolutionConfigError {
    ZeroGenerations,
    PopulationTooSmall(usize),
    OddPopulation(usize),
    ZeroSwissRounds,
    TooManySwissRounds {
        rounds: usize,
        population: usize,
    },
    TooManyElites {
        elites: usize,
        population: usize,
    },
    InvalidParentCandidateCount {
        candidates: usize,
        population: usize,
    },
    InvalidProbability {
        name: &'static str,
        value: f64,
    },
    InvalidMutationStep {
        name: &'static str,
        value: f64,
    },
    AnchorWeightOutOfRange(u8),
    InconsistentAnchorConfiguration {
        weight_percent: u8,
        opening_pairs: usize,
    },
    AnchorScoreOverflow {
        swiss_rounds: usize,
        opening_pairs: usize,
    },
    ConflictingTrainingObjectives,
}

impl fmt::Display for EvolutionConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl Error for EvolutionConfigError {}

#[derive(Clone, Debug, PartialEq)]
pub struct Individual {
    id: IndividualId,
    genome: Genome,
}

impl Individual {
    pub fn new(id: IndividualId, genome: Genome) -> Self {
        Self { id, genome }
    }
    pub const fn id(&self) -> IndividualId {
        self.id
    }
    pub const fn genome(&self) -> &Genome {
        &self.genome
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluatedIndividual {
    individual: Individual,
    fitness: FitnessScore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScoreComponent {
    half_points: Score,
    available_half_points: u32,
}

impl ScoreComponent {
    pub const fn new(half_points: Score, available_half_points: u32) -> Self {
        Self {
            half_points,
            available_half_points,
        }
    }
    pub const fn half_points(self) -> Score {
        self.half_points
    }
    pub const fn available_half_points(self) -> u32 {
        self.available_half_points
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FitnessScore {
    selection_units: Score,
    maximum_selection_units: u32,
    self_play: ScoreComponent,
    default_anchor: Option<ScoreComponent>,
    historical: Option<ScoreComponent>,
}

impl FitnessScore {
    pub const fn legacy(half_points: Score) -> Self {
        Self {
            selection_units: half_points,
            maximum_selection_units: 0,
            self_play: ScoreComponent::new(half_points, 0),
            default_anchor: None,
            historical: None,
        }
    }
    pub const fn new(
        selection_units: Score,
        maximum_selection_units: u32,
        self_play: ScoreComponent,
        default_anchor: Option<ScoreComponent>,
    ) -> Self {
        Self {
            selection_units,
            maximum_selection_units,
            self_play,
            default_anchor,
            historical: None,
        }
    }
    pub const fn with_historical(
        selection_units: Score,
        maximum_selection_units: u32,
        self_play: ScoreComponent,
        historical: ScoreComponent,
    ) -> Self {
        Self {
            selection_units,
            maximum_selection_units,
            self_play,
            default_anchor: None,
            historical: Some(historical),
        }
    }
    pub const fn selection_units(self) -> Score {
        self.selection_units
    }
    pub const fn maximum_selection_units(self) -> u32 {
        self.maximum_selection_units
    }
    pub const fn self_play(self) -> ScoreComponent {
        self.self_play
    }
    pub const fn default_anchor(self) -> Option<ScoreComponent> {
        self.default_anchor
    }
    pub const fn historical(self) -> Option<ScoreComponent> {
        self.historical
    }
    pub const fn is_legacy(self) -> bool {
        self.maximum_selection_units == 0
    }

    pub fn reconstruct_selection_units(
        self,
        swiss_rounds: usize,
        anchor: DefaultAnchorConfig,
    ) -> Option<Score> {
        if self.is_legacy() {
            return None;
        }
        if self.self_play.available_half_points != (swiss_rounds * 4) as u32 {
            return None;
        }
        match (anchor.enabled(), self.default_anchor) {
            (false, None) => Some(self.self_play.half_points),
            (true, Some(component))
                if component.available_half_points == (anchor.opening_pairs() * 4) as u32 =>
            {
                Some(anchored_selection_score(
                    self.self_play.half_points,
                    component.half_points,
                    swiss_rounds,
                    anchor,
                ))
            }
            _ => None,
        }
    }
}

impl EvaluatedIndividual {
    pub fn new(individual: Individual, fitness: Score) -> Self {
        Self {
            individual,
            fitness: FitnessScore::legacy(fitness),
        }
    }
    pub fn with_fitness(individual: Individual, fitness: FitnessScore) -> Self {
        Self {
            individual,
            fitness,
        }
    }
    pub const fn individual(&self) -> &Individual {
        &self.individual
    }
    pub const fn fitness(&self) -> Score {
        self.fitness.selection_units()
    }
    pub const fn fitness_score(&self) -> FitnessScore {
        self.fitness
    }
}

/// Selects the two parents. The implementation name deliberately avoids
/// "tournament", which is reserved for chess competition in this crate.
pub trait ParentSelector {
    fn select_pair(
        &mut self,
        population: &[EvaluatedIndividual],
        rng: &mut dyn RandomSource,
    ) -> Result<(usize, usize), ReproductionError>;
}

#[derive(Clone, Debug)]
pub struct CompetitiveParentSelector {
    candidate_count: usize,
}

impl CompetitiveParentSelector {
    pub fn new(candidate_count: usize) -> Self {
        Self { candidate_count }
    }

    fn select_one(
        &self,
        population: &[EvaluatedIndividual],
        excluded: Option<usize>,
        rng: &mut dyn RandomSource,
    ) -> Result<usize, ReproductionError> {
        if population.len() < 2 {
            return Err(ReproductionError::TooFewParents);
        }
        if self.candidate_count == 0 {
            return Err(ReproductionError::ZeroParentCandidates);
        }
        let mut available: Vec<_> = (0..population.len())
            .filter(|index| Some(*index) != excluded)
            .collect();
        if self.candidate_count > available.len() {
            return Err(ReproductionError::TooManyParentCandidates {
                candidates: self.candidate_count,
                available: available.len(),
            });
        }
        let mut best = None;
        for offset in 0..self.candidate_count {
            let selected = offset + rng.index(available.len() - offset);
            available.swap(offset, selected);
            let candidate = available[offset];
            best = match best {
                None => Some(candidate),
                Some(current) if is_fitter(&population[candidate], &population[current]) => {
                    Some(candidate)
                }
                Some(current) => Some(current),
            };
        }
        best.ok_or(ReproductionError::ZeroParentCandidates)
    }
}

impl ParentSelector for CompetitiveParentSelector {
    fn select_pair(
        &mut self,
        population: &[EvaluatedIndividual],
        rng: &mut dyn RandomSource,
    ) -> Result<(usize, usize), ReproductionError> {
        let first = self.select_one(population, None, rng)?;
        let second = self.select_one(population, Some(first), rng)?;
        Ok((first, second))
    }
}

pub trait CrossoverOperator {
    fn crossover(
        &mut self,
        first: &Genome,
        second: &Genome,
        rng: &mut dyn RandomSource,
    ) -> Result<Genome, ReproductionError>;
}

#[derive(Default)]
pub struct BlendCrossover;

impl CrossoverOperator for BlendCrossover {
    fn crossover(
        &mut self,
        first: &Genome,
        second: &Genome,
        rng: &mut dyn RandomSource,
    ) -> Result<Genome, ReproductionError> {
        let mut genes = [0.0; GENE_COUNT];
        for (index, gene) in genes.iter_mut().enumerate() {
            let alpha = rng.unit_f64();
            *gene = alpha * first.genes()[index] + (1.0 - alpha) * second.genes()[index];
        }
        Genome::new(genes).map_err(ReproductionError::InvalidGenome)
    }
}

pub trait MutationOperator {
    fn mutate(
        &mut self,
        genome: &Genome,
        rng: &mut dyn RandomSource,
    ) -> Result<Genome, ReproductionError>;
}

#[derive(Clone, Debug)]
pub struct AdditiveMutation {
    probability: f64,
    strong_probability: f64,
    step: f64,
    strong_step: f64,
}

impl AdditiveMutation {
    pub fn from_config(config: &EvolutionConfig) -> Self {
        Self {
            probability: config.gene_mutation_probability(),
            strong_probability: config.strong_mutation_probability(),
            step: config.mutation_step(),
            strong_step: config.strong_mutation_step(),
        }
    }
}

impl MutationOperator for AdditiveMutation {
    fn mutate(
        &mut self,
        genome: &Genome,
        rng: &mut dyn RandomSource,
    ) -> Result<Genome, ReproductionError> {
        let mut genes = *genome.genes();
        for gene in &mut genes {
            if rng.unit_f64() < self.probability {
                let magnitude = if rng.unit_f64() < self.strong_probability {
                    self.strong_step
                } else {
                    self.step
                };
                let delta = (rng.unit_f64() * 2.0 - 1.0) * magnitude;
                *gene = (*gene + delta).max(0.0);
            }
        }
        if genes.iter().all(|gene| *gene == 0.0) {
            let index = genome
                .genes()
                .iter()
                .enumerate()
                .max_by(|left, right| left.1.total_cmp(right.1))
                .map(|(index, _)| index)
                .expect("a genome always contains genes");
            genes[index] = genome.genes()[index];
        }
        Genome::new(genes).map_err(ReproductionError::InvalidGenome)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReproductionError {
    TooFewParents,
    ZeroParentCandidates,
    TooManyParentCandidates { candidates: usize, available: usize },
    InvalidParentIndex(usize),
    InvalidGenome(GenomeError),
}

impl fmt::Display for ReproductionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl Error for ReproductionError {}

/// Boundary between evolution and the mechanism that obtains fitness.
pub trait PopulationEvaluator {
    type Error;

    fn evaluate(
        &mut self,
        generation: usize,
        population: &[Individual],
        config: &EvolutionConfig,
    ) -> Result<Vec<Standing>, Self::Error>;

    fn evaluate_with_progress(
        &mut self,
        generation: usize,
        population: &[Individual],
        config: &EvolutionConfig,
        _observer: &mut dyn ProgressObserver,
    ) -> Result<Vec<Standing>, Self::Error> {
        self.evaluate(generation, population, config)
    }

    fn evaluate_with_history(
        &mut self,
        generation: usize,
        population: &[Individual],
        archive: &HistoricalArchive,
        config: &EvolutionConfig,
        observer: &mut dyn ProgressObserver,
    ) -> Result<Vec<Standing>, Self::Error> {
        let _ = archive;
        self.evaluate_with_progress(generation, population, config, observer)
    }

    fn historical_audit(&self) -> HistoricalAudit {
        HistoricalAudit::default()
    }

    fn fitness_score(
        &self,
        _individual: IndividualId,
        standing: Score,
        _config: &EvolutionConfig,
    ) -> FitnessScore {
        FitnessScore::legacy(standing)
    }
}

pub struct SelfPlayPopulationEvaluator<E> {
    executor: E,
    last_self_play_scores: BTreeMap<IndividualId, Score>,
    last_anchor_scores: Option<BTreeMap<IndividualId, Score>>,
    last_historical_scores: Option<BTreeMap<IndividualId, Score>>,
    last_historical_available: u32,
    last_historical_audit: HistoricalAudit,
}

impl<R> SelfPlayPopulationEvaluator<SequentialRoundExecutor<R>> {
    pub fn new(runner: R) -> Self {
        Self {
            executor: SequentialRoundExecutor::new(runner),
            last_self_play_scores: BTreeMap::new(),
            last_anchor_scores: None,
            last_historical_scores: None,
            last_historical_available: 0,
            last_historical_audit: HistoricalAudit::default(),
        }
    }

    pub fn runner(&self) -> &R {
        self.executor.runner()
    }
}

impl<F> SelfPlayPopulationEvaluator<ParallelRoundExecutor<F>> {
    pub fn parallel(factory: F, workers: NonZeroUsize) -> Self {
        Self {
            executor: ParallelRoundExecutor::new(factory, workers),
            last_self_play_scores: BTreeMap::new(),
            last_anchor_scores: None,
            last_historical_scores: None,
            last_historical_available: 0,
            last_historical_audit: HistoricalAudit::default(),
        }
    }
}

impl<E: RoundExecutor + AuxiliaryRoundExecutor> PopulationEvaluator
    for SelfPlayPopulationEvaluator<E>
{
    type Error = SelfPlayEvaluationError<E::Error>;

    fn evaluate(
        &mut self,
        generation: usize,
        population: &[Individual],
        config: &EvolutionConfig,
    ) -> Result<Vec<Standing>, Self::Error> {
        self.evaluate_with_progress(generation, population, config, &mut NoopProgressObserver)
    }

    fn evaluate_with_progress(
        &mut self,
        generation: usize,
        population: &[Individual],
        config: &EvolutionConfig,
        observer: &mut dyn ProgressObserver,
    ) -> Result<Vec<Standing>, Self::Error> {
        let seed = derive_seed(config.training().master_seed(), generation as u64, 0);
        let training = config.training().with_master_seed(seed);
        let openings = OpeningPool::generate(config.swiss_rounds(), &training)
            .map_err(SelfPlayEvaluationError::Opening)?;
        let genomes: BTreeMap<_, _> = population
            .iter()
            .map(|individual| (individual.id(), individual.genome().clone()))
            .collect();
        let mut standings: Vec<_> = population
            .iter()
            .map(|individual| Standing {
                individual: individual.id(),
                score: Score(0),
            })
            .collect();
        let mut scheduler = SwissScheduler::new(
            population.iter().map(Individual::id),
            derive_seed(seed, 1, 0),
        )
        .map_err(SelfPlayEvaluationError::Pairing)?;
        let mut generation_games =
            Vec::with_capacity(config.population_size() * config.swiss_rounds());

        for (round_index, opening) in openings.openings().iter().enumerate() {
            let round = scheduler
                .next_round(&standings, opening.id)
                .map_err(SelfPlayEvaluationError::Pairing)?;
            let records = self
                .executor
                .play_round(&round, &genomes, opening, &training)
                .map_err(SelfPlayEvaluationError::Round)?;
            let statistics = GameStatistics::from_records(
                records
                    .iter()
                    .flat_map(|record| [&record.first_game, &record.second_game]),
            );
            generation_games.extend(records.iter().flat_map(|record| {
                [
                    (record.first_game.outcome, record.first_game.moves.len()),
                    (record.second_game.outcome, record.second_game.moves.len()),
                ]
            }));
            let scores: BTreeMap<_, _> = records
                .into_iter()
                .flat_map(|record| {
                    [
                        (record.pairing.a, record.a_score),
                        (record.pairing.b, record.b_score),
                    ]
                })
                .collect();
            for standing in &mut standings {
                standing.score.0 += scores[&standing.individual].0;
            }
            observer.on_event(ProgressEvent::SelfPlayRoundCompleted {
                generation,
                round: round_index,
                total_rounds: openings.openings().len(),
                opening: opening.id,
                statistics,
            });
        }
        observer.on_event(ProgressEvent::SelfPlayGenerationCompleted {
            generation,
            statistics: GameStatistics::from_outcomes_and_plies(generation_games),
        });
        self.last_self_play_scores = standings
            .iter()
            .map(|standing| (standing.individual, standing.score))
            .collect();
        self.last_anchor_scores = None;
        let anchor = config.default_anchor();
        if anchor.enabled() {
            let condition_seed = derive_seed(
                config.training().master_seed(),
                u64::from(anchor.weight_percent()),
                anchor.opening_pairs() as u64,
            );
            let anchor_seed = derive_seed(condition_seed, generation as u64, ANCHOR_SEED_DOMAIN);
            let anchor_training = config.training().with_master_seed(anchor_seed);
            let anchor_openings = OpeningPool::generate(anchor.opening_pairs(), &anchor_training)
                .map_err(SelfPlayEvaluationError::Opening)?;
            let candidates = population
                .iter()
                .map(|individual| (individual.id(), individual.genome().to_evaluation_config()))
                .collect::<Vec<_>>();
            let mut anchor_scores: BTreeMap<_, Score> = population
                .iter()
                .map(|individual| (individual.id(), Score(0)))
                .collect();
            let mut anchor_games = Vec::with_capacity(
                population
                    .len()
                    .saturating_mul(anchor.opening_pairs())
                    .saturating_mul(2),
            );
            for opening in anchor_openings.openings() {
                let records = self
                    .executor
                    .play_default_anchor_round(&candidates, opening, &anchor_training)
                    .map_err(SelfPlayEvaluationError::Round)?;
                for record in records {
                    anchor_scores
                        .get_mut(&record.candidate)
                        .expect("every anchor candidate belongs to the population")
                        .0 += record.candidate_score.0;
                    anchor_games.extend([
                        (record.first_game.outcome, record.first_game.moves.len()),
                        (record.second_game.outcome, record.second_game.moves.len()),
                    ]);
                }
            }
            let candidate_half_points = anchor_scores.values().map(|score| score.0).sum();
            observer.on_event(ProgressEvent::DefaultAnchorCompleted {
                generation,
                opening_pairs: anchor.opening_pairs(),
                games: anchor_games.len(),
                candidate_half_points,
                available_half_points: (population.len() * anchor.opening_pairs() * 4) as u32,
                maximum_selection_units: 400
                    * config.swiss_rounds() as u32
                    * anchor.opening_pairs() as u32,
                statistics: GameStatistics::from_outcomes_and_plies(anchor_games),
            });
            self.last_anchor_scores = Some(anchor_scores.clone());
            for standing in &mut standings {
                standing.score = anchored_selection_score(
                    standing.score,
                    anchor_scores[&standing.individual],
                    config.swiss_rounds(),
                    anchor,
                );
            }
        }
        Ok(standings)
    }

    fn evaluate_with_history(
        &mut self,
        generation: usize,
        population: &[Individual],
        archive: &HistoricalArchive,
        config: &EvolutionConfig,
        observer: &mut dyn ProgressObserver,
    ) -> Result<Vec<Standing>, Self::Error> {
        let mut standings =
            self.evaluate_with_progress(generation, population, config, observer)?;
        self.last_historical_scores = None;
        self.last_historical_available = 0;
        self.last_historical_audit = HistoricalAudit {
            distinct_phenotypes: population
                .iter()
                .map(|individual| phenotype_fingerprint(individual.genome()))
                .collect::<BTreeSet<_>>()
                .len(),
            archive_size_before: archive.entries().len(),
            archive_size_after: archive.entries().len(),
            ..HistoricalAudit::default()
        };
        let historical = config.historical();
        let sampled = archive.sample(
            historical.opponents(),
            config.training().master_seed(),
            generation,
        );
        if !historical.enabled() || sampled.is_empty() {
            return Ok(standings);
        }
        let seed = derive_seed(
            config.training().master_seed(),
            generation as u64,
            0x4849_5354_4f50_454e,
        );
        let training = config.training().with_master_seed(seed);
        let openings = OpeningPool::generate(historical.opening_pairs(), &training)
            .map_err(SelfPlayEvaluationError::Opening)?;
        let candidates = population
            .iter()
            .map(|individual| (individual.id(), individual.genome().clone()))
            .collect::<Vec<_>>();
        let opponents = sampled
            .iter()
            .map(|entry| (entry.champion().id(), entry.champion().genome().clone()))
            .collect::<Vec<_>>();
        let mut scores = population
            .iter()
            .map(|individual| (individual.id(), Score(0)))
            .collect::<BTreeMap<_, _>>();
        for opening in openings.openings() {
            for record in self
                .executor
                .play_historical_round(&candidates, &opponents, opening, &training)
                .map_err(SelfPlayEvaluationError::Round)?
            {
                scores
                    .get_mut(&record.candidate)
                    .expect("candidate exists")
                    .0 += record.candidate_score.0;
            }
        }
        let available = (sampled.len() * historical.opening_pairs() * 4) as u32;
        self.last_historical_available = available;
        self.last_historical_scores = Some(scores.clone());
        self.last_historical_audit.opponent_generations =
            sampled.iter().map(|entry| entry.generation()).collect();
        self.last_historical_audit.opponent_ids =
            sampled.iter().map(|entry| entry.champion().id()).collect();
        self.last_historical_audit.opening_ids = openings
            .openings()
            .iter()
            .map(|opening| opening.id)
            .collect();
        for standing in &mut standings {
            standing.score = historical_selection_score(
                self.last_self_play_scores[&standing.individual],
                scores[&standing.individual],
                (config.swiss_rounds() * 4) as u32,
                available,
                historical.weight_percent(),
            );
        }
        Ok(standings)
    }

    fn historical_audit(&self) -> HistoricalAudit {
        self.last_historical_audit.clone()
    }

    fn fitness_score(
        &self,
        individual: IndividualId,
        standing: Score,
        config: &EvolutionConfig,
    ) -> FitnessScore {
        let self_play = self.last_self_play_scores[&individual];
        let self_available = (config.swiss_rounds() * 4) as u32;
        let anchor = config.default_anchor();
        if config.historical().enabled() && self.last_historical_available > 0 {
            let historical = self
                .last_historical_scores
                .as_ref()
                .expect("historical scores recorded")[&individual];
            return FitnessScore::with_historical(
                standing,
                10_000,
                ScoreComponent::new(self_play, self_available),
                ScoreComponent::new(historical, self.last_historical_available),
            );
        }
        if !anchor.enabled() {
            return FitnessScore::new(
                standing,
                self_available,
                ScoreComponent::new(self_play, self_available),
                None,
            );
        }
        let anchor_available = (anchor.opening_pairs() * 4) as u32;
        FitnessScore::new(
            standing,
            400 * config.swiss_rounds() as u32 * anchor.opening_pairs() as u32,
            ScoreComponent::new(self_play, self_available),
            Some(ScoreComponent::new(
                self.last_anchor_scores
                    .as_ref()
                    .expect("anchored evaluation records anchor scores")[&individual],
                anchor_available,
            )),
        )
    }
}

pub fn historical_selection_score(
    contemporary: Score,
    historical: Score,
    contemporary_available: u32,
    historical_available: u32,
    historical_weight_percent: u8,
) -> Score {
    if historical_available == 0 || historical_weight_percent == 0 {
        return contemporary;
    }
    let contemporary_weight = 100 - u32::from(historical_weight_percent);
    let units = u64::from(contemporary.0) * u64::from(contemporary_weight) * 100
        / u64::from(contemporary_available)
        + u64::from(historical.0) * u64::from(historical_weight_percent) * 100
            / u64::from(historical_available);
    Score(units as u32)
}

pub fn anchored_selection_score(
    self_play: Score,
    anchor: Score,
    swiss_rounds: usize,
    config: DefaultAnchorConfig,
) -> Score {
    if !config.enabled() {
        return self_play;
    }
    let self_weight = 100_u64 - u64::from(config.weight_percent());
    let weighted = u64::from(self_play.0) * self_weight * config.opening_pairs() as u64
        + u64::from(anchor.0) * u64::from(config.weight_percent()) * swiss_rounds as u64;
    Score(u32::try_from(weighted).expect("validated evolution dimensions fit a score"))
}

#[derive(Debug)]
pub enum SelfPlayEvaluationError<E> {
    Opening(OpeningGenerationError),
    Pairing(PairingError),
    Round(RoundExecutionError<E>),
}

impl<E: fmt::Display> fmt::Display for SelfPlayEvaluationError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Opening(source) => write!(formatter, "opening generation failed: {source}"),
            Self::Pairing(source) => write!(formatter, "pairing failed: {source}"),
            Self::Round(source) => write!(formatter, "round execution failed: {source}"),
        }
    }
}
impl<E: Error + 'static> Error for SelfPlayEvaluationError<E> {}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationResult {
    index: usize,
    ranked: Vec<EvaluatedIndividual>,
    pub(crate) historical_audit: HistoricalAudit,
}

impl GenerationResult {
    pub fn new(
        index: usize,
        ranked: Vec<EvaluatedIndividual>,
    ) -> Result<Self, EvolutionStateError> {
        if ranked.is_empty() {
            return Err(EvolutionStateError::EmptyRanking);
        }
        Ok(Self {
            index,
            ranked,
            historical_audit: HistoricalAudit::default(),
        })
    }
    pub const fn index(&self) -> usize {
        self.index
    }
    pub fn ranked(&self) -> &[EvaluatedIndividual] {
        &self.ranked
    }
    pub fn best(&self) -> &EvaluatedIndividual {
        &self.ranked[0]
    }
    pub fn historical_audit(&self) -> &HistoricalAudit {
        &self.historical_audit
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvolutionResult {
    generations: Vec<GenerationResult>,
    best_ever: EvaluatedIndividual,
}

impl EvolutionResult {
    pub fn new(
        generations: Vec<GenerationResult>,
        best_ever: EvaluatedIndividual,
    ) -> Result<Self, EvolutionStateError> {
        if generations.is_empty() {
            return Err(EvolutionStateError::NoCompletedGenerations);
        }
        Ok(Self {
            generations,
            best_ever,
        })
    }
    pub fn generations(&self) -> &[GenerationResult] {
        &self.generations
    }
    pub const fn best_ever(&self) -> &EvaluatedIndividual {
        &self.best_ever
    }
}

/// Everything required to continue immediately after a completed generation.
#[derive(Clone, Debug, PartialEq)]
pub struct EvolutionState {
    next_generation: usize,
    population: Vec<Individual>,
    generations: Vec<GenerationResult>,
    best_ever: EvaluatedIndividual,
    next_id: u64,
    rng_state: u64,
    archive: HistoricalArchive,
}

impl EvolutionState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        next_generation: usize,
        population: Vec<Individual>,
        generations: Vec<GenerationResult>,
        best_ever: EvaluatedIndividual,
        next_id: u64,
        rng_state: u64,
    ) -> Result<Self, EvolutionStateError> {
        if next_generation == 0 || generations.len() != next_generation {
            return Err(EvolutionStateError::GenerationMismatch);
        }
        if generations
            .iter()
            .enumerate()
            .any(|(index, generation)| generation.index() != index)
        {
            return Err(EvolutionStateError::NonContiguousHistory);
        }
        let maximum_id = population
            .iter()
            .map(|individual| individual.id().0)
            .chain(generations.iter().flat_map(|generation| {
                generation
                    .ranked()
                    .iter()
                    .map(|individual| individual.individual().id().0)
            }))
            .max()
            .unwrap_or(0);
        if next_id <= maximum_id {
            return Err(EvolutionStateError::InvalidNextId);
        }
        Self::new_with_archive(
            next_generation,
            population,
            generations,
            best_ever,
            next_id,
            rng_state,
            HistoricalArchive::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_archive(
        next_generation: usize,
        population: Vec<Individual>,
        generations: Vec<GenerationResult>,
        best_ever: EvaluatedIndividual,
        next_id: u64,
        rng_state: u64,
        archive: HistoricalArchive,
    ) -> Result<Self, EvolutionStateError> {
        if next_generation == 0 || generations.len() != next_generation {
            return Err(EvolutionStateError::GenerationMismatch);
        }
        if generations
            .iter()
            .enumerate()
            .any(|(index, generation)| generation.index() != index)
        {
            return Err(EvolutionStateError::NonContiguousHistory);
        }
        let maximum_id = population
            .iter()
            .map(|individual| individual.id().0)
            .chain(generations.iter().flat_map(|generation| {
                generation
                    .ranked()
                    .iter()
                    .map(|individual| individual.individual().id().0)
            }))
            .max()
            .unwrap_or(0);
        if next_id <= maximum_id {
            return Err(EvolutionStateError::InvalidNextId);
        }
        Ok(Self {
            next_generation,
            population,
            generations,
            best_ever,
            next_id,
            rng_state,
            archive,
        })
    }

    pub const fn next_generation(&self) -> usize {
        self.next_generation
    }
    pub fn population(&self) -> &[Individual] {
        &self.population
    }
    pub fn generations(&self) -> &[GenerationResult] {
        &self.generations
    }
    pub const fn best_ever(&self) -> &EvaluatedIndividual {
        &self.best_ever
    }
    pub const fn next_id(&self) -> u64 {
        self.next_id
    }
    pub const fn rng_state(&self) -> u64 {
        self.rng_state
    }
    pub const fn archive(&self) -> &HistoricalArchive {
        &self.archive
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvolutionStateError {
    EmptyRanking,
    NoCompletedGenerations,
    GenerationMismatch,
    NonContiguousHistory,
    InvalidNextId,
}

impl fmt::Display for EvolutionStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for EvolutionStateError {}

pub struct EvolutionEngine<E> {
    config: EvolutionConfig,
    evaluator: E,
    selector: Box<dyn ParentSelector>,
    crossover: Box<dyn CrossoverOperator>,
    mutation: Box<dyn MutationOperator>,
    rng: Box<dyn RandomSource>,
    observer: Box<dyn ProgressObserver>,
    next_id: u64,
}

impl<E> EvolutionEngine<E> {
    pub fn with_defaults(config: EvolutionConfig, evaluator: E) -> Self {
        Self::with_observer(config, evaluator, Box::new(NoopProgressObserver))
    }

    pub fn with_observer(
        config: EvolutionConfig,
        evaluator: E,
        observer: Box<dyn ProgressObserver>,
    ) -> Self {
        let seed = derive_seed(config.training().master_seed(), u64::MAX, 0);
        let selector = CompetitiveParentSelector::new(config.parent_candidate_count());
        let mutation = AdditiveMutation::from_config(&config);
        Self::with_operators_and_observer(
            config,
            evaluator,
            Box::new(selector),
            Box::new(BlendCrossover),
            Box::new(mutation),
            Box::new(StableRng::new(seed)),
            observer,
        )
    }

    pub fn with_operators(
        config: EvolutionConfig,
        evaluator: E,
        selector: Box<dyn ParentSelector>,
        crossover: Box<dyn CrossoverOperator>,
        mutation: Box<dyn MutationOperator>,
        rng: Box<dyn RandomSource>,
    ) -> Self {
        Self::with_operators_and_observer(
            config,
            evaluator,
            selector,
            crossover,
            mutation,
            rng,
            Box::new(NoopProgressObserver),
        )
    }

    pub fn with_operators_and_observer(
        config: EvolutionConfig,
        evaluator: E,
        selector: Box<dyn ParentSelector>,
        crossover: Box<dyn CrossoverOperator>,
        mutation: Box<dyn MutationOperator>,
        rng: Box<dyn RandomSource>,
        observer: Box<dyn ProgressObserver>,
    ) -> Self {
        Self {
            config,
            evaluator,
            selector,
            crossover,
            mutation,
            rng,
            observer,
            next_id: 0,
        }
    }

    pub fn initialize_population(&mut self) -> Vec<Individual> {
        (0..self.config.population_size())
            .map(|_| {
                let mut genes = [0.0; GENE_COUNT];
                for gene in &mut genes {
                    *gene = self.rng.unit_f64();
                }
                if genes.iter().all(|gene| *gene == 0.0) {
                    let index = self.rng.index(GENE_COUNT);
                    genes[index] = 1.0;
                }
                let genome = Genome::new(genes).expect("random genes are finite and non-negative");
                self.new_individual(genome)
            })
            .collect()
    }

    fn new_individual(&mut self, genome: Genome) -> Individual {
        let individual = Individual::new(IndividualId(self.next_id), genome);
        self.next_id += 1;
        individual
    }
}

impl<E: PopulationEvaluator> EvolutionEngine<E> {
    pub fn run(&mut self) -> Result<EvolutionResult, EvolutionError<E::Error>> {
        let population = self.initialize_population();
        self.run_from(population)
    }

    pub fn run_from(
        &mut self,
        population: Vec<Individual>,
    ) -> Result<EvolutionResult, EvolutionError<E::Error>> {
        self.run_internal(
            population,
            0,
            Vec::new(),
            None,
            HistoricalArchive::default(),
            false,
            |_| Ok(()),
        )
    }

    /// Resumes a persisted run and exposes a consistent state after each newly
    /// completed generation. The callback is never invoked for generations
    /// already present in `state`.
    pub fn run_resuming<F>(
        &mut self,
        state: EvolutionState,
        checkpoint: F,
    ) -> Result<EvolutionResult, EvolutionError<E::Error>>
    where
        F: FnMut(&EvolutionState) -> Result<(), Box<dyn Error + Send + Sync>>,
    {
        if state.next_generation > self.config.generations() {
            return Err(EvolutionError::CompletedGenerationsExceedTarget {
                completed: state.next_generation,
                target: self.config.generations(),
            });
        }
        self.next_id = state.next_id;
        if !self.rng.restore_persistent_state(state.rng_state) {
            return Err(EvolutionError::RandomSourceNotPersistent);
        }
        self.run_internal(
            state.population,
            state.next_generation,
            state.generations,
            Some(state.best_ever),
            state.archive,
            true,
            checkpoint,
        )
    }

    /// Starts a new run and publishes a resumable state after every generation.
    pub fn run_with_checkpoints<F>(
        &mut self,
        checkpoint: F,
    ) -> Result<EvolutionResult, EvolutionError<E::Error>>
    where
        F: FnMut(&EvolutionState) -> Result<(), Box<dyn Error + Send + Sync>>,
    {
        let population = self.initialize_population();
        self.run_internal(
            population,
            0,
            Vec::new(),
            None,
            HistoricalArchive::default(),
            true,
            checkpoint,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn run_internal<F>(
        &mut self,
        mut population: Vec<Individual>,
        start_generation: usize,
        mut generations: Vec<GenerationResult>,
        mut best_ever: Option<EvaluatedIndividual>,
        mut archive: HistoricalArchive,
        publish_checkpoints: bool,
        mut checkpoint: F,
    ) -> Result<EvolutionResult, EvolutionError<E::Error>>
    where
        F: FnMut(&EvolutionState) -> Result<(), Box<dyn Error + Send + Sync>>,
    {
        validate_population(&population, self.config.population_size())?;
        if let Some(maximum_id) = population.iter().map(|individual| individual.id().0).max() {
            self.next_id = self.next_id.max(maximum_id + 1);
        }

        self.observer.on_event(ProgressEvent::EvolutionStarted {
            generations: self.config.generations(),
            population_size: self.config.population_size(),
        });
        generations.reserve(self.config.generations().saturating_sub(generations.len()));
        for generation in start_generation..self.config.generations() {
            self.observer.on_event(ProgressEvent::GenerationStarted {
                generation,
                total_generations: self.config.generations(),
            });
            let standings = self
                .evaluator
                .evaluate_with_history(
                    generation,
                    &population,
                    &archive,
                    &self.config,
                    self.observer.as_mut(),
                )
                .map_err(EvolutionError::Evaluation)?;
            let fitness_scores: BTreeMap<_, _> = standings
                .iter()
                .map(|standing| {
                    (
                        standing.individual,
                        self.evaluator.fitness_score(
                            standing.individual,
                            standing.score,
                            &self.config,
                        ),
                    )
                })
                .collect();
            let ranked = rank_population_with_scores(
                &population,
                standings,
                &fitness_scores,
                derive_seed(
                    self.config.training().master_seed(),
                    generation as u64,
                    0x5449_4542_5245_414b,
                ),
            )?;
            if self.config.historical().enabled()
                || best_ever
                    .as_ref()
                    .is_none_or(|best| is_fitter(&ranked[0], best))
            {
                best_ever = Some(ranked[0].clone());
            }
            generations.push(GenerationResult {
                index: generation,
                ranked: ranked.clone(),
                historical_audit: self.evaluator.historical_audit(),
            });
            archive.insert_champion(generation, &ranked[0], self.config.historical());
            generations
                .last_mut()
                .expect("generation was just appended")
                .historical_audit
                .archive_size_after = archive.entries().len();
            self.observer.on_event(ProgressEvent::GenerationCompleted {
                generation,
                total_generations: self.config.generations(),
                best: ranked[0].individual().id(),
                best_score: ranked[0].fitness(),
            });
            if generation + 1 < self.config.generations() {
                population = self.next_generation(&ranked)?;
            }
            if publish_checkpoints {
                let rng_state = self
                    .rng
                    .persistent_state()
                    .ok_or(EvolutionError::RandomSourceNotPersistent)?;
                let state = EvolutionState::new_with_archive(
                    generation + 1,
                    population.clone(),
                    generations.clone(),
                    best_ever.clone().expect("this generation produced a best"),
                    self.next_id,
                    rng_state,
                    archive.clone(),
                )
                .expect("engine produces a valid resumable state");
                checkpoint(&state).map_err(EvolutionError::Checkpoint)?;
            }
        }
        let best_ever = best_ever.expect("at least one generation is configured or resumed");
        self.observer.on_event(ProgressEvent::EvolutionCompleted {
            generations: self.config.generations(),
            best: best_ever.individual().id(),
            best_score: best_ever.fitness(),
        });
        Ok(EvolutionResult {
            generations,
            best_ever,
        })
    }

    fn next_generation(
        &mut self,
        ranked: &[EvaluatedIndividual],
    ) -> Result<Vec<Individual>, EvolutionError<E::Error>> {
        let mut next: Vec<_> = ranked
            .iter()
            .take(self.config.elite_count())
            .map(|evaluated| evaluated.individual().clone())
            .collect();
        let mut fingerprints: BTreeSet<_> = next
            .iter()
            .map(|individual| genome_fingerprint(individual.genome()))
            .collect();
        while next.len() < self.config.population_size() {
            for attempt in 0..=OFFSPRING_DUPLICATE_RETRIES {
                let offspring = self.reproduce(ranked)?;
                let fingerprint = genome_fingerprint(&offspring);
                if fingerprints.insert(fingerprint) || attempt == OFFSPRING_DUPLICATE_RETRIES {
                    next.push(self.new_individual(offspring));
                    break;
                }
            }
        }
        Ok(next)
    }

    fn reproduce(
        &mut self,
        ranked: &[EvaluatedIndividual],
    ) -> Result<Genome, EvolutionError<E::Error>> {
        let (first, second) = self
            .selector
            .select_pair(ranked, self.rng.as_mut())
            .map_err(EvolutionError::Reproduction)?;
        let first = ranked.get(first).ok_or(EvolutionError::Reproduction(
            ReproductionError::InvalidParentIndex(first),
        ))?;
        let second = ranked.get(second).ok_or(EvolutionError::Reproduction(
            ReproductionError::InvalidParentIndex(second),
        ))?;
        let crossed = self
            .crossover
            .crossover(
                first.individual().genome(),
                second.individual().genome(),
                self.rng.as_mut(),
            )
            .map_err(EvolutionError::Reproduction)?;
        self.mutation
            .mutate(&crossed, self.rng.as_mut())
            .map_err(EvolutionError::Reproduction)
    }
}

fn genome_fingerprint(genome: &Genome) -> [u64; GENE_COUNT] {
    genome.genes().map(f64::to_bits)
}

fn validate_population<E>(
    population: &[Individual],
    expected: usize,
) -> Result<(), EvolutionError<E>> {
    if population.len() != expected {
        return Err(EvolutionError::InvalidPopulationSize {
            expected,
            actual: population.len(),
        });
    }
    let ids: BTreeSet<_> = population.iter().map(Individual::id).collect();
    if ids.len() != population.len() {
        return Err(EvolutionError::DuplicateIndividualId);
    }
    Ok(())
}

#[cfg(test)]
fn rank_population<E>(
    population: &[Individual],
    standings: Vec<Standing>,
) -> Result<Vec<EvaluatedIndividual>, EvolutionError<E>> {
    let fitness_scores = standings
        .iter()
        .map(|standing| (standing.individual, FitnessScore::legacy(standing.score)))
        .collect();
    rank_population_with_scores(population, standings, &fitness_scores, 0)
}

fn rank_population_with_scores<E>(
    population: &[Individual],
    standings: Vec<Standing>,
    fitness_scores: &BTreeMap<IndividualId, FitnessScore>,
    tie_seed: u64,
) -> Result<Vec<EvaluatedIndividual>, EvolutionError<E>> {
    if standings.len() != population.len() {
        return Err(EvolutionError::InvalidStandings);
    }
    let scores: BTreeMap<_, _> = standings
        .into_iter()
        .map(|standing| (standing.individual, standing.score))
        .collect();
    if scores.len() != population.len()
        || population
            .iter()
            .any(|individual| !scores.contains_key(&individual.id()))
    {
        return Err(EvolutionError::InvalidStandings);
    }
    let mut ranked: Vec<_> = population
        .iter()
        .map(|individual| {
            EvaluatedIndividual::with_fitness(individual.clone(), fitness_scores[&individual.id()])
        })
        .collect();
    let tie_keys: BTreeMap<_, _> = population
        .iter()
        .enumerate()
        .map(|(index, individual)| (individual.id(), derive_seed(tie_seed, index as u64, 0)))
        .collect();
    ranked.sort_by(|left, right| {
        right.fitness().cmp(&left.fitness()).then_with(|| {
            tie_keys[&left.individual().id()].cmp(&tie_keys[&right.individual().id()])
        })
    });
    Ok(ranked)
}

fn is_fitter(left: &EvaluatedIndividual, right: &EvaluatedIndividual) -> bool {
    left.fitness() > right.fitness()
}

#[derive(Debug)]
pub enum EvolutionError<E> {
    Evaluation(E),
    InvalidPopulationSize { expected: usize, actual: usize },
    DuplicateIndividualId,
    InvalidStandings,
    Reproduction(ReproductionError),
    CompletedGenerationsExceedTarget { completed: usize, target: usize },
    RandomSourceNotPersistent,
    Checkpoint(Box<dyn Error + Send + Sync>),
}

impl<E: fmt::Display> fmt::Display for EvolutionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(source) => write!(formatter, "population evaluation failed: {source}"),
            Self::InvalidPopulationSize { expected, actual } => write!(
                formatter,
                "population size must be {expected}, got {actual}"
            ),
            Self::DuplicateIndividualId => {
                formatter.write_str("population contains duplicate individual ids")
            }
            Self::InvalidStandings => {
                formatter.write_str("evaluator returned standings for a different population")
            }
            Self::Reproduction(source) => write!(formatter, "reproduction failed: {source}"),
            Self::CompletedGenerationsExceedTarget { completed, target } => write!(
                formatter,
                "checkpoint contains {completed} completed generations, target is {target}"
            ),
            Self::RandomSourceNotPersistent => {
                formatter.write_str("random source does not support deterministic persistence")
            }
            Self::Checkpoint(source) => write!(formatter, "checkpoint failed: {source}"),
        }
    }
}
impl<E: Error + 'static> Error for EvolutionError<E> {}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use crate::{
        encounter::{ConfiguredGameRunner, GameRunner},
        self_play::{DrawReason, GameOutcome, GameRecord},
    };

    #[derive(Default)]
    struct RecordingDrawRunner {
        calls: Vec<(Genome, Genome, u64)>,
        configured_calls: Vec<(
            blocky_chess::EvaluationConfig,
            blocky_chess::EvaluationConfig,
            u64,
        )>,
    }

    impl ConfiguredGameRunner for RecordingDrawRunner {
        type Error = std::convert::Infallible;

        fn play_configured(
            &mut self,
            white: blocky_chess::EvaluationConfig,
            black: blocky_chess::EvaluationConfig,
            opening: &crate::openings::Opening,
            _search_depth: usize,
            _max_game_plies: usize,
        ) -> Result<GameRecord, Self::Error> {
            self.configured_calls.push((white, black, opening.seed));
            Ok(GameRecord {
                outcome: GameOutcome::Draw(DrawReason::MaxPlies),
                moves: Vec::new(),
                position_history: vec![opening.position.clone()],
                final_position: opening.position.clone(),
            })
        }
    }

    impl GameRunner for RecordingDrawRunner {
        type Error = std::convert::Infallible;

        fn play(
            &mut self,
            white: &Genome,
            black: &Genome,
            opening: &crate::openings::Opening,
            _search_depth: usize,
            _max_game_plies: usize,
        ) -> Result<GameRecord, Self::Error> {
            self.calls
                .push((white.clone(), black.clone(), opening.seed));
            Ok(GameRecord {
                outcome: GameOutcome::Draw(DrawReason::MaxPlies),
                moves: Vec::new(),
                position_history: vec![opening.position.clone()],
                final_position: opening.position.clone(),
            })
        }
    }

    fn config(generations: usize, elite_count: usize) -> EvolutionConfig {
        EvolutionConfig::new(
            TrainingConfig::default(),
            generations,
            4,
            3,
            elite_count,
            2,
            0.15,
            0.02,
            0.1,
            0.5,
        )
        .unwrap()
    }

    #[test]
    fn proposed_values_are_defaults_and_every_value_is_exposed() {
        let config = EvolutionConfig::default();
        assert_eq!(config.training().search_depth(), 4);
        assert_eq!(config.generations(), 100);
        assert_eq!(config.population_size(), 32);
        assert_eq!(config.swiss_rounds(), 5);
        assert_eq!(config.elite_count(), 2);
        assert_eq!(config.parent_candidate_count(), 3);
        assert_eq!(config.gene_mutation_probability(), 0.15);
        assert_eq!(config.strong_mutation_probability(), 0.02);
        assert_eq!(config.mutation_step(), 0.1);
        assert_eq!(config.strong_mutation_step(), 0.5);
        assert_eq!(config.default_anchor(), DefaultAnchorConfig::default());
    }

    #[test]
    fn default_anchor_configuration_rejects_inconsistent_or_out_of_range_values() {
        assert!(matches!(
            DefaultAnchorConfig::new(101, 1),
            Err(EvolutionConfigError::AnchorWeightOutOfRange(101))
        ));
        for (weight, pairs) in [(0, 1), (10, 0)] {
            assert!(matches!(
                DefaultAnchorConfig::new(weight, pairs),
                Err(EvolutionConfigError::InconsistentAnchorConfiguration { .. })
            ));
        }
        assert_eq!(
            DefaultAnchorConfig::new(10, 1).unwrap().weight_percent(),
            10
        );
        assert!(matches!(
            config(1, 1).with_default_anchor(DefaultAnchorConfig::new(10, usize::MAX).unwrap()),
            Err(EvolutionConfigError::AnchorScoreOverflow { .. })
        ));
    }

    #[test]
    fn anchored_ranking_uses_exact_normalized_integer_scores_and_preserves_ties() {
        let anchor = DefaultAnchorConfig::new(10, 1).unwrap();
        assert_eq!(
            anchored_selection_score(Score(10), Score(2), 5, anchor),
            Score(1000)
        );
        assert_eq!(
            anchored_selection_score(Score(5), Score(1), 5, anchor),
            Score(500)
        );
        assert_eq!(
            anchored_selection_score(Score(7), Score(99), 5, DefaultAnchorConfig::default()),
            Score(7)
        );
    }

    #[test]
    fn historical_fitness_normalizes_full_partial_and_empty_archives() {
        let full = historical_selection_score(Score(10), Score(4), 20, 8, 30);
        let partial = historical_selection_score(Score(10), Score(2), 20, 4, 30);
        assert_eq!(full, partial);
        assert_eq!(full, Score(5_000));
        assert_eq!(
            historical_selection_score(Score(10), Score(0), 20, 0, 30),
            Score(10)
        );
    }

    #[test]
    fn historical_evaluation_shares_opponent_and_opening_and_reverses_colors_without_default() {
        let historical = HistoricalConfig::new(30, 1, 1, 1, 4).unwrap();
        let config = config(2, 1).with_historical(historical).unwrap();
        let population = (0..4)
            .map(|id| {
                let mut genes = [0.05; GENE_COUNT];
                genes[id] = 1.0;
                Individual::new(IndividualId(id as u64), Genome::new(genes).unwrap())
            })
            .collect::<Vec<_>>();
        let mut opponent_genes = [0.02; GENE_COUNT];
        opponent_genes[8] = 1.0;
        let opponent = Individual::new(IndividualId(99), Genome::new(opponent_genes).unwrap());
        let champion = EvaluatedIndividual::new(opponent.clone(), Score(0));
        let mut archive = HistoricalArchive::default();
        archive.insert_champion(0, &champion, historical);
        let mut evaluator = SelfPlayPopulationEvaluator::new(RecordingDrawRunner::default());
        evaluator
            .evaluate_with_history(1, &population, &archive, &config, &mut NoopProgressObserver)
            .unwrap();

        let calls = &evaluator.runner().calls;
        let historical_calls = &calls[calls.len() - population.len() * 2..];
        for (candidate, pair) in population.iter().zip(historical_calls.chunks_exact(2)) {
            assert_eq!(pair[0].0, *candidate.genome());
            assert_eq!(pair[0].1, *opponent.genome());
            assert_eq!(pair[1].0, *opponent.genome());
            assert_eq!(pair[1].1, *candidate.genome());
            assert_eq!(pair[0].2, pair[1].2);
        }
        assert_eq!(
            evaluator.historical_audit().opponent_ids,
            vec![IndividualId(99)]
        );
        assert_eq!(evaluator.historical_audit().opening_ids.len(), 1);
    }

    #[test]
    fn deterministic_tie_lottery_does_not_always_favour_the_lowest_id() {
        let population = (0..4)
            .map(|id| {
                let mut genes = [0.1; GENE_COUNT];
                genes[id] = 1.0;
                Individual::new(IndividualId(id as u64), Genome::new(genes).unwrap())
            })
            .collect::<Vec<_>>();
        let standings = population
            .iter()
            .map(|individual| Standing {
                individual: individual.id(),
                score: Score(2),
            })
            .collect::<Vec<_>>();
        let fitness = standings
            .iter()
            .map(|standing| (standing.individual, FitnessScore::legacy(standing.score)))
            .collect::<BTreeMap<_, _>>();
        let winners = (0..64)
            .map(|seed| {
                rank_population_with_scores::<std::convert::Infallible>(
                    &population,
                    standings.clone(),
                    &fitness,
                    seed,
                )
                .unwrap()[0]
                    .individual()
                    .id()
            })
            .collect::<BTreeSet<_>>();
        assert!(winners.len() > 1);
        assert!(winners.contains(&IndividualId(0)));
        assert!(winners.iter().any(|id| *id != IndividualId(0)));
    }

    #[test]
    fn anchor_uses_literal_default_with_color_swap_and_an_isolated_opening_stream() {
        let training = TrainingConfig::new(1, 1, 77, 0..=0, 10).unwrap();
        let base = EvolutionConfig::new(training, 1, 4, 1, 1, 2, 0.15, 0.02, 0.1, 0.5).unwrap();
        let anchored = base
            .clone()
            .with_default_anchor(DefaultAnchorConfig::new(10, 1).unwrap())
            .unwrap();
        let population = (0..4)
            .map(|id| {
                let mut genes = [1.0; GENE_COUNT];
                genes[0] = (id + 1) as f64 / 10.0;
                Individual::new(IndividualId(id), Genome::new(genes).unwrap())
            })
            .collect::<Vec<_>>();

        let mut plain_evaluator = SelfPlayPopulationEvaluator::new(RecordingDrawRunner::default());
        let plain = plain_evaluator.evaluate(0, &population, &base).unwrap();
        let plain_calls = plain_evaluator.runner().calls.clone();

        let mut anchored_evaluator =
            SelfPlayPopulationEvaluator::new(RecordingDrawRunner::default());
        let ranked = anchored_evaluator
            .evaluate(0, &population, &anchored)
            .unwrap();
        let calls = &anchored_evaluator.runner().calls;
        let configured_calls = &anchored_evaluator.runner().configured_calls;

        assert_eq!(calls, &plain_calls);
        assert_eq!(configured_calls.len(), population.len() * 2);
        assert!(plain.iter().all(|standing| standing.score == Score(2)));
        assert!(ranked.iter().all(|standing| standing.score == Score(200)));
        for standing in &ranked {
            let fitness =
                anchored_evaluator.fitness_score(standing.individual, standing.score, &anchored);
            assert_eq!(fitness.self_play(), ScoreComponent::new(Score(2), 4));
            assert_eq!(
                fitness.default_anchor(),
                Some(ScoreComponent::new(Score(2), 4))
            );
            assert_eq!(fitness.maximum_selection_units(), 400);
            assert_eq!(
                fitness.reconstruct_selection_units(1, anchored.default_anchor()),
                Some(standing.score)
            );
        }
        for (candidate, pair) in population.iter().zip(configured_calls.chunks_exact(2)) {
            assert_eq!(
                (&pair[0].0, &pair[0].1),
                (
                    &candidate.genome().to_evaluation_config(),
                    &blocky_chess::EvaluationConfig::default()
                )
            );
            assert_eq!(
                (&pair[1].0, &pair[1].1),
                (
                    &blocky_chess::EvaluationConfig::default(),
                    &candidate.genome().to_evaluation_config()
                )
            );
            assert_eq!(pair[0].2, pair[1].2);
            assert_ne!(pair[0].2, plain_calls[0].2);
        }
    }

    #[test]
    fn invalid_hyperparameters_are_typed() {
        let make = |generations, population, rounds, elites, candidates, probability, step| {
            EvolutionConfig::new(
                TrainingConfig::default(),
                generations,
                population,
                rounds,
                elites,
                candidates,
                probability,
                0.02,
                step,
                0.5,
            )
        };
        assert!(matches!(
            make(0, 4, 1, 1, 2, 0.1, 0.1),
            Err(EvolutionConfigError::ZeroGenerations)
        ));
        assert!(matches!(
            make(1, 3, 1, 1, 2, 0.1, 0.1),
            Err(EvolutionConfigError::OddPopulation(3))
        ));
        assert!(matches!(
            make(1, 4, 4, 1, 2, 0.1, 0.1),
            Err(EvolutionConfigError::TooManySwissRounds { .. })
        ));
        assert!(matches!(
            make(1, 4, 1, 4, 2, 0.1, 0.1),
            Err(EvolutionConfigError::TooManyElites { .. })
        ));
        assert!(matches!(
            make(1, 4, 1, 1, 0, 0.1, 0.1),
            Err(EvolutionConfigError::InvalidParentCandidateCount { .. })
        ));
        assert!(matches!(
            make(1, 4, 1, 1, 2, f64::NAN, 0.1),
            Err(EvolutionConfigError::InvalidProbability { .. })
        ));
        assert!(matches!(
            make(1, 4, 1, 1, 2, 0.1, 0.0),
            Err(EvolutionConfigError::InvalidMutationStep { .. })
        ));
    }

    #[derive(Clone)]
    struct ByIdEvaluator {
        seen: Rc<RefCell<Vec<Vec<Individual>>>>,
    }

    impl PopulationEvaluator for ByIdEvaluator {
        type Error = std::convert::Infallible;
        fn evaluate(
            &mut self,
            _: usize,
            population: &[Individual],
            _: &EvolutionConfig,
        ) -> Result<Vec<Standing>, Self::Error> {
            self.seen.borrow_mut().push(population.to_vec());
            Ok(population
                .iter()
                .map(|individual| Standing {
                    individual: individual.id(),
                    score: Score(u32::MAX - individual.id().0 as u32),
                })
                .collect())
        }
    }

    #[derive(Clone)]
    struct RecordingObserver(Rc<RefCell<Vec<ProgressEvent>>>);

    impl ProgressObserver for RecordingObserver {
        fn on_event(&mut self, event: ProgressEvent) {
            self.0.borrow_mut().push(event);
        }
    }

    #[test]
    fn seeded_runs_are_deterministic_and_initial_population_is_random_knowledge_free() {
        let seen_a = Rc::new(RefCell::new(vec![]));
        let seen_b = Rc::new(RefCell::new(vec![]));
        let mut a = EvolutionEngine::with_defaults(
            config(2, 1),
            ByIdEvaluator {
                seen: seen_a.clone(),
            },
        );
        let mut b = EvolutionEngine::with_defaults(
            config(2, 1),
            ByIdEvaluator {
                seen: seen_b.clone(),
            },
        );
        let result_a = a.run().unwrap();
        let result_b = b.run().unwrap();
        assert_eq!(result_a, result_b);
        assert_eq!(*seen_a.borrow(), *seen_b.borrow());
        assert!(seen_a.borrow()[0]
            .iter()
            .all(|individual| individual.genome() != &Genome::default()));
    }

    #[test]
    fn progress_reports_generation_boundaries_without_changing_the_result() {
        let silent_seen = Rc::new(RefCell::new(vec![]));
        let observed_seen = Rc::new(RefCell::new(vec![]));
        let events = Rc::new(RefCell::new(vec![]));
        let mut silent =
            EvolutionEngine::with_defaults(config(2, 1), ByIdEvaluator { seen: silent_seen });
        let mut observed = EvolutionEngine::with_observer(
            config(2, 1),
            ByIdEvaluator {
                seen: observed_seen,
            },
            Box::new(RecordingObserver(events.clone())),
        );

        let silent_result = silent.run().unwrap();
        let observed_result = observed.run().unwrap();

        assert_eq!(observed_result, silent_result);
        let events = events.borrow();
        assert_eq!(
            events
                .iter()
                .map(std::mem::discriminant)
                .collect::<Vec<_>>(),
            [
                ProgressEvent::EvolutionStarted {
                    generations: 0,
                    population_size: 0,
                },
                ProgressEvent::GenerationStarted {
                    generation: 0,
                    total_generations: 0,
                },
                ProgressEvent::GenerationCompleted {
                    generation: 0,
                    total_generations: 0,
                    best: IndividualId(0),
                    best_score: Score(0),
                },
                ProgressEvent::GenerationStarted {
                    generation: 0,
                    total_generations: 0,
                },
                ProgressEvent::GenerationCompleted {
                    generation: 0,
                    total_generations: 0,
                    best: IndividualId(0),
                    best_score: Score(0),
                },
                ProgressEvent::EvolutionCompleted {
                    generations: 0,
                    best: IndividualId(0),
                    best_score: Score(0),
                },
            ]
            .iter()
            .map(std::mem::discriminant)
            .collect::<Vec<_>>()
        );
        assert!(matches!(
            events.first(),
            Some(ProgressEvent::EvolutionStarted {
                generations: 2,
                population_size: 4
            })
        ));
        assert!(matches!(
            events.last(),
            Some(ProgressEvent::EvolutionCompleted { generations: 2, .. })
        ));
    }

    struct DrawRunner;

    impl GameRunner for DrawRunner {
        type Error = std::convert::Infallible;

        fn play(
            &mut self,
            _white: &Genome,
            _black: &Genome,
            opening: &crate::openings::Opening,
            _search_depth: usize,
            _max_game_plies: usize,
        ) -> Result<crate::self_play::GameRecord, Self::Error> {
            Ok(crate::self_play::GameRecord {
                outcome: crate::self_play::GameOutcome::Draw(
                    crate::self_play::DrawReason::MaxPlies,
                ),
                moves: vec![],
                position_history: vec![opening.position.clone()],
                final_position: opening.position.clone(),
            })
        }
    }

    impl ConfiguredGameRunner for DrawRunner {
        type Error = std::convert::Infallible;

        fn play_configured(
            &mut self,
            _white: blocky_chess::EvaluationConfig,
            _black: blocky_chess::EvaluationConfig,
            opening: &crate::openings::Opening,
            _search_depth: usize,
            _max_game_plies: usize,
        ) -> Result<GameRecord, Self::Error> {
            Ok(GameRecord {
                outcome: GameOutcome::Draw(DrawReason::MaxPlies),
                moves: vec![],
                position_history: vec![opening.position.clone()],
                final_position: opening.position.clone(),
            })
        }
    }

    #[test]
    fn self_play_reports_each_completed_swiss_round() {
        let events = Rc::new(RefCell::new(vec![]));
        let observer = RecordingObserver(events.clone());
        let mut engine = EvolutionEngine::with_observer(
            config(1, 1),
            SelfPlayPopulationEvaluator::new(DrawRunner),
            Box::new(observer),
        );

        engine.run().unwrap();

        let round_events = events
            .borrow()
            .iter()
            .copied()
            .filter(|event| matches!(event, ProgressEvent::SelfPlayRoundCompleted { .. }))
            .collect::<Vec<_>>();
        assert_eq!(round_events.len(), 3);
        for (round, event) in round_events.iter().enumerate() {
            assert!(matches!(
                event,
                ProgressEvent::SelfPlayRoundCompleted {
                    generation: 0,
                    round: actual_round,
                    total_rounds: 3,
                    ..
                } if *actual_round == round
            ));
        }
    }

    #[test]
    fn production_self_play_is_identical_for_one_and_many_workers() {
        let training = TrainingConfig::new(1, 1, 77, 2..=2, 100).unwrap();
        let configuration = EvolutionConfig::new(training, 2, 4, 2, 1, 2, 0.15, 0.02, 0.1, 0.5)
            .unwrap()
            .with_historical(HistoricalConfig::new(30, 1, 1, 1, 4).unwrap())
            .unwrap();
        let mut sequential = EvolutionEngine::with_defaults(
            configuration.clone(),
            SelfPlayPopulationEvaluator::parallel(
                crate::encounter::ProductionGameRunner,
                NonZeroUsize::new(1).unwrap(),
            ),
        );
        let mut parallel = EvolutionEngine::with_defaults(
            configuration,
            SelfPlayPopulationEvaluator::parallel(
                crate::encounter::ProductionGameRunner,
                NonZeroUsize::new(4).unwrap(),
            ),
        );
        let mut sequential_states = vec![];
        let mut parallel_states = vec![];

        let sequential_result = sequential
            .run_with_checkpoints(|state| {
                sequential_states.push(state.clone());
                Ok(())
            })
            .unwrap();
        let parallel_result = parallel
            .run_with_checkpoints(|state| {
                parallel_states.push(state.clone());
                Ok(())
            })
            .unwrap();

        assert_eq!(parallel_result, sequential_result);
        assert_eq!(parallel_states, sequential_states);
    }

    #[test]
    fn elites_keep_their_identity_and_genome_while_children_get_unique_ids() {
        let seen = Rc::new(RefCell::new(vec![]));
        let mut engine =
            EvolutionEngine::with_defaults(config(2, 2), ByIdEvaluator { seen: seen.clone() });
        engine.run().unwrap();
        let populations = seen.borrow();
        let first = &populations[0];
        let second = &populations[1];
        assert_eq!(second[0], first[0]);
        assert_eq!(second[1], first[1]);
        let all_ids: BTreeSet<_> = second.iter().map(Individual::id).collect();
        assert_eq!(all_ids.len(), second.len());
        assert!(second[2..].iter().all(|child| child.id().0 >= 4));
        for individual in second {
            assert_eq!(
                individual
                    .genome()
                    .genes()
                    .iter()
                    .copied()
                    .fold(0.0, f64::max),
                1.0
            );
            assert!(individual.genome().genes().iter().all(|gene| *gene >= 0.0));
        }
    }

    struct SequenceRng {
        values: std::collections::VecDeque<u64>,
    }
    impl RandomSource for SequenceRng {
        fn next_u64(&mut self) -> u64 {
            self.values.pop_front().unwrap_or(u64::MAX / 2)
        }
    }

    #[test]
    fn competitive_parent_selection_samples_distinct_candidates() {
        let population = (0..4)
            .map(|id| {
                EvaluatedIndividual::new(
                    Individual::new(IndividualId(id), Genome::default()),
                    Score(id as u32),
                )
            })
            .collect::<Vec<_>>();
        let mut selector = CompetitiveParentSelector::new(2);
        let mut rng = SequenceRng {
            values: [0, 0, 0, 0].into(),
        };

        let (first, second) = selector.select_pair(&population, &mut rng).unwrap();

        assert_eq!(first, 1);
        assert_ne!(first, second);
    }

    #[test]
    fn additive_mutation_can_revive_a_zero_gene() {
        let mut genes = [1.0; GENE_COUNT];
        genes[0] = 0.0;
        let genome = Genome::new(genes).unwrap();
        let mut mutation = AdditiveMutation {
            probability: 1.0,
            strong_probability: 0.0,
            step: 0.5,
            strong_step: 1.0,
        };
        // Mutation chance, regular-strength choice and a positive delta,
        // repeatedly.
        let mut rng = SequenceRng {
            values: (0..GENE_COUNT)
                .flat_map(|_| [0, u64::MAX - 1, u64::MAX - 1])
                .collect(),
        };
        let mutated = mutation.mutate(&genome, &mut rng).unwrap();
        assert!(mutated.genes()[0] > 0.0);
    }

    #[test]
    fn strong_mutation_probability_does_not_increase_gene_mutation_probability() {
        let genome = Genome::new([1.0; GENE_COUNT]).unwrap();
        let mut mutation = AdditiveMutation {
            probability: 0.0,
            strong_probability: 1.0,
            step: 0.1,
            strong_step: 1.0,
        };
        let mut rng = StableRng::new(3);

        assert_eq!(mutation.mutate(&genome, &mut rng).unwrap(), genome);
    }

    struct FirstTwoSelector;
    impl ParentSelector for FirstTwoSelector {
        fn select_pair(
            &mut self,
            _: &[EvaluatedIndividual],
            _: &mut dyn RandomSource,
        ) -> Result<(usize, usize), ReproductionError> {
            Ok((0, 1))
        }
    }
    struct FirstParentCrossover;
    impl CrossoverOperator for FirstParentCrossover {
        fn crossover(
            &mut self,
            first: &Genome,
            _: &Genome,
            _: &mut dyn RandomSource,
        ) -> Result<Genome, ReproductionError> {
            Ok(first.clone())
        }
    }
    struct NoMutation;
    impl MutationOperator for NoMutation {
        fn mutate(
            &mut self,
            genome: &Genome,
            _: &mut dyn RandomSource,
        ) -> Result<Genome, ReproductionError> {
            Ok(genome.clone())
        }
    }

    #[test]
    fn operators_are_substitutable_and_drive_generation_transition() {
        let seen = Rc::new(RefCell::new(vec![]));
        let mut engine = EvolutionEngine::with_operators(
            config(2, 1),
            ByIdEvaluator { seen: seen.clone() },
            Box::new(FirstTwoSelector),
            Box::new(FirstParentCrossover),
            Box::new(NoMutation),
            Box::new(StableRng::new(8)),
        );
        engine.run().unwrap();
        let populations = seen.borrow();
        assert!(populations[1]
            .iter()
            .all(|individual| individual.genome() == populations[0][0].genome()));
    }

    struct DuplicateThenDistinctMutation {
        calls: usize,
    }
    impl MutationOperator for DuplicateThenDistinctMutation {
        fn mutate(
            &mut self,
            genome: &Genome,
            _: &mut dyn RandomSource,
        ) -> Result<Genome, ReproductionError> {
            self.calls += 1;
            if self.calls % 2 == 1 {
                return Ok(genome.clone());
            }
            let mut genes = *genome.genes();
            genes[self.calls / 2 - 1] = 0.0;
            Genome::new(genes).map_err(ReproductionError::InvalidGenome)
        }
    }

    #[test]
    fn retries_duplicate_offspring_to_preserve_diversity() {
        let seen = Rc::new(RefCell::new(vec![]));
        let mut engine = EvolutionEngine::with_operators(
            config(2, 1),
            ByIdEvaluator { seen: seen.clone() },
            Box::new(FirstTwoSelector),
            Box::new(FirstParentCrossover),
            Box::new(DuplicateThenDistinctMutation { calls: 0 }),
            Box::new(StableRng::new(8)),
        );

        engine.run().unwrap();

        let populations = seen.borrow();
        let fingerprints: BTreeSet<_> = populations[1]
            .iter()
            .map(|individual| genome_fingerprint(individual.genome()))
            .collect();
        assert_eq!(fingerprints.len(), populations[1].len());
    }

    #[test]
    fn rejects_duplicate_or_surplus_standings_even_if_all_individuals_are_present() {
        let population = (0..4)
            .map(|id| Individual::new(IndividualId(id), Genome::default()))
            .collect::<Vec<_>>();
        let standings = [
            Standing {
                individual: IndividualId(0),
                score: Score(0),
            },
            Standing {
                individual: IndividualId(1),
                score: Score(0),
            },
            Standing {
                individual: IndividualId(2),
                score: Score(0),
            },
            Standing {
                individual: IndividualId(3),
                score: Score(0),
            },
            Standing {
                individual: IndividualId(0),
                score: Score(10),
            },
        ];

        assert!(matches!(
            rank_population::<std::convert::Infallible>(&population, standings.to_vec()),
            Err(EvolutionError::InvalidStandings)
        ));
    }

    #[test]
    fn blend_crossover_stays_between_parents_per_gene() {
        let first = Genome::new([1.0; GENE_COUNT]).unwrap();
        let mut second_genes = [0.2; GENE_COUNT];
        second_genes[0] = 1.0;
        let second = Genome::new(second_genes).unwrap();
        let mut crossover = BlendCrossover;
        let mut rng = StableRng::new(1);
        let child = crossover.crossover(&first, &second, &mut rng).unwrap();
        for index in 0..GENE_COUNT {
            assert!(child.genes()[index] >= second.genes()[index]);
            assert!(child.genes()[index] <= first.genes()[index]);
        }
    }

    #[test]
    fn resumed_run_is_bit_for_bit_equal_and_does_not_repeat_completed_generations() {
        let configuration = config(4, 1)
            .with_historical(HistoricalConfig::new(30, 1, 1, 1, 3).unwrap())
            .unwrap();
        let baseline_seen = Rc::new(RefCell::new(vec![]));
        let mut baseline = EvolutionEngine::with_defaults(
            configuration.clone(),
            ByIdEvaluator {
                seen: baseline_seen,
            },
        );
        let expected = baseline.run().unwrap();

        let interrupted_seen = Rc::new(RefCell::new(vec![]));
        let mut interrupted = EvolutionEngine::with_defaults(
            configuration.clone(),
            ByIdEvaluator {
                seen: interrupted_seen.clone(),
            },
        );
        let captured = Rc::new(RefCell::new(None));
        let capture = captured.clone();
        let result = interrupted.run_with_checkpoints(|state| {
            if state.next_generation() == 2 {
                *capture.borrow_mut() = Some(state.clone());
                return Err(Box::new(std::io::Error::other("simulated interruption")));
            }
            Ok(())
        });
        assert!(matches!(result, Err(EvolutionError::Checkpoint(_))));
        assert_eq!(interrupted_seen.borrow().len(), 2);
        assert_eq!(
            captured
                .borrow()
                .as_ref()
                .unwrap()
                .archive()
                .entries()
                .len(),
            1
        );

        let resumed_seen = Rc::new(RefCell::new(vec![]));
        let mut resumed = EvolutionEngine::with_defaults(
            configuration,
            ByIdEvaluator {
                seen: resumed_seen.clone(),
            },
        );
        let actual = resumed
            .run_resuming(captured.borrow_mut().take().unwrap(), |_| Ok(()))
            .unwrap();

        assert_eq!(actual, expected);
        assert_eq!(resumed_seen.borrow().len(), 2);
    }

    #[test]
    fn historical_self_play_resume_is_exactly_equal_to_uninterrupted_execution() {
        let configuration = config(3, 1)
            .with_historical(HistoricalConfig::new(30, 2, 1, 1, 3).unwrap())
            .unwrap();
        let mut baseline = EvolutionEngine::with_defaults(
            configuration.clone(),
            SelfPlayPopulationEvaluator::new(DrawRunner),
        );
        let expected = baseline.run().unwrap();

        let mut interrupted = EvolutionEngine::with_defaults(
            configuration.clone(),
            SelfPlayPopulationEvaluator::new(DrawRunner),
        );
        let mut captured = None;
        let result = interrupted.run_with_checkpoints(|state| {
            if state.next_generation() == 1 {
                captured = Some(state.clone());
                return Err(Box::new(std::io::Error::other("interrupt")));
            }
            Ok(())
        });
        assert!(matches!(result, Err(EvolutionError::Checkpoint(_))));

        let mut resumed = EvolutionEngine::with_defaults(
            configuration,
            SelfPlayPopulationEvaluator::new(DrawRunner),
        );
        let actual = resumed.run_resuming(captured.unwrap(), |_| Ok(())).unwrap();
        assert_eq!(actual, expected);
    }
}
