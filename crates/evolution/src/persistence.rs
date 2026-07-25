//! Versioned, validated persistence for resumable training and experiment results.

use std::{
    collections::BTreeSet,
    error::Error,
    fmt, fs, io,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    evolution::{
        DefaultAnchorConfig, EvaluatedIndividual, EvolutionConfig, EvolutionState,
        EvolutionStateError, FitnessScore, GenerationResult, Individual, ScoreComponent,
    },
    experiment::ExperimentReport,
    genome::{Genome, GENE_COUNT},
    pairing::{IndividualId, Score},
    self_play::{DrawReason, GameOutcome},
    telemetry::{GameObservation, GameStatistics},
    training::TrainingConfig,
    validation::{CandidateSelector, ValidationConfig},
};

pub const PERSISTENCE_FORMAT: &str = "blocky-evolution";
pub const PERSISTENCE_VERSION: u32 = 2;
const LEGACY_PERSISTENCE_VERSION: u32 = 1;

#[derive(Debug)]
pub enum PersistenceError {
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    InvalidJson(serde_json::Error),
    WrongFormat(String),
    UnsupportedVersion(u32),
    IncompatibleEvolutionConfig,
    CorruptData(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "could not {operation} {}: {source}",
                path.display()
            ),
            Self::InvalidJson(source) => write!(formatter, "invalid JSON: {source}"),
            Self::WrongFormat(format) => {
                write!(formatter, "unexpected persistence format `{format}`")
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported persistence version {version}")
            }
            Self::IncompatibleEvolutionConfig => {
                formatter.write_str("checkpoint evolution configuration is incompatible")
            }
            Self::CorruptData(reason) => write!(formatter, "corrupt persistence data: {reason}"),
        }
    }
}

impl Error for PersistenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidJson(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointDocument {
    format: String,
    version: u32,
    evolution_config: EvolutionConfigData,
    state: EvolutionStateData,
}

pub fn write_checkpoint(
    path: &Path,
    config: &EvolutionConfig,
    state: &EvolutionState,
) -> Result<(), PersistenceError> {
    let document = CheckpointDocument {
        format: PERSISTENCE_FORMAT.to_owned(),
        version: PERSISTENCE_VERSION,
        evolution_config: EvolutionConfigData::from(config),
        state: EvolutionStateData::from(state),
    };
    write_json_atomically(path, &document)
}

pub fn read_checkpoint(
    path: &Path,
    expected_config: &EvolutionConfig,
) -> Result<EvolutionState, PersistenceError> {
    let bytes = fs::read(path).map_err(|source| io_error("read", path, source))?;
    let document: CheckpointDocument =
        serde_json::from_slice(&bytes).map_err(PersistenceError::InvalidJson)?;
    verify_header(&document.format, document.version)?;
    if document.evolution_config != EvolutionConfigData::from(expected_config) {
        return Err(PersistenceError::IncompatibleEvolutionConfig);
    }
    let state: EvolutionState = document.state.try_into()?;
    validate_checkpoint_state(&state, expected_config)?;
    Ok(state)
}

/// Reads a checkpoint together with the configuration embedded in it.
pub fn read_checkpoint_unchecked_config(
    path: &Path,
) -> Result<(EvolutionConfig, EvolutionState), PersistenceError> {
    let bytes = fs::read(path).map_err(|source| io_error("read", path, source))?;
    let document: CheckpointDocument =
        serde_json::from_slice(&bytes).map_err(PersistenceError::InvalidJson)?;
    verify_header(&document.format, document.version)?;
    let config: EvolutionConfig = document.evolution_config.try_into()?;
    let state: EvolutionState = document.state.try_into()?;
    validate_checkpoint_state(&state, &config)?;
    Ok((config, state))
}

fn validate_checkpoint_state(
    state: &EvolutionState,
    config: &EvolutionConfig,
) -> Result<(), PersistenceError> {
    if state.next_generation() > config.generations() {
        return Err(PersistenceError::CorruptData(
            "completed generations exceed configured target".into(),
        ));
    }
    for evaluated in state
        .generations()
        .iter()
        .flat_map(|generation| generation.ranked())
        .chain(std::iter::once(state.best_ever()))
    {
        let fitness = evaluated.fitness_score();
        if fitness.is_legacy() {
            if config.default_anchor().enabled() {
                return Err(PersistenceError::CorruptData(
                    "anchored checkpoint contains legacy unaudited fitness".into(),
                ));
            }
            continue;
        }
        let reconstructed =
            fitness.reconstruct_selection_units(config.swiss_rounds(), config.default_anchor());
        if reconstructed != Some(fitness.selection_units()) {
            return Err(PersistenceError::CorruptData(
                "selection score does not match its persisted components".into(),
            ));
        }
    }
    let validate_individuals = |individuals: Vec<&Individual>| {
        if individuals.len() != config.population_size()
            || individuals
                .iter()
                .map(|individual| individual.id())
                .collect::<BTreeSet<_>>()
                .len()
                != individuals.len()
        {
            return Err(PersistenceError::CorruptData(
                "population size or identifiers are invalid".into(),
            ));
        }
        Ok(())
    };
    validate_individuals(state.population().iter().collect())?;
    for generation in state.generations() {
        validate_individuals(
            generation
                .ranked()
                .iter()
                .map(EvaluatedIndividual::individual)
                .collect(),
        )?;
    }
    if !state
        .generations()
        .iter()
        .any(|generation| generation.ranked().contains(state.best_ever()))
    {
        return Err(PersistenceError::CorruptData(
            "historical best is absent from generation history".into(),
        ));
    }
    Ok(())
}

pub fn write_experiment_report(
    path: &Path,
    evolution_config: &EvolutionConfig,
    report: &ExperimentReport,
) -> Result<(), PersistenceError> {
    let document = ExperimentReportDocument {
        format: PERSISTENCE_FORMAT.to_owned(),
        version: PERSISTENCE_VERSION,
        evolution_config: EvolutionConfigData::from(evolution_config),
        validation_config: ValidationConfigData::from(&report.validation().config),
        generations: report
            .evolution()
            .generations()
            .iter()
            .map(GenerationData::from)
            .collect(),
        champion: EvaluatedIndividualData::from(report.evolution().best_ever()),
        validation: ValidationData::from(report),
    };
    write_json_atomically(path, &document)
}

fn verify_header(format: &str, version: u32) -> Result<(), PersistenceError> {
    if format != PERSISTENCE_FORMAT {
        return Err(PersistenceError::WrongFormat(format.to_owned()));
    }
    if version != PERSISTENCE_VERSION && version != LEGACY_PERSISTENCE_VERSION {
        return Err(PersistenceError::UnsupportedVersion(version));
    }
    Ok(())
}

pub(crate) fn write_json_atomically<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), PersistenceError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|source| io_error("create directory for", path, source))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| PersistenceError::CorruptData("output path has no file name".into()))?
        .to_string_lossy();
    let temporary = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    let backup = parent.join(format!(".{file_name}.{}.bak", std::process::id()));
    let bytes = serde_json::to_vec_pretty(value).map_err(PersistenceError::InvalidJson)?;
    let mut temporary_file = fs::File::create(&temporary)
        .map_err(|source| io_error("create temporary file for", path, source))?;
    if let Err(source) = temporary_file
        .write_all(&bytes)
        .and_then(|()| temporary_file.sync_all())
    {
        drop(temporary_file);
        let _ = fs::remove_file(&temporary);
        return Err(io_error("write temporary file for", path, source));
    }
    drop(temporary_file);

    if path.exists() {
        fs::rename(path, &backup)
            .map_err(|source| io_error("prepare replacement of", path, source))?;
    }
    if let Err(source) = fs::rename(&temporary, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temporary);
        return Err(io_error("replace", path, source));
    }
    if backup.exists() {
        fs::remove_file(&backup).map_err(|source| io_error("remove backup for", path, source))?;
    }
    Ok(())
}

#[derive(Serialize)]
struct StandaloneBenchmarkDocument<'a> {
    format: &'static str,
    version: u32,
    training_seed: u64,
    selector: StandaloneSelectorData,
    candidate: EvaluatedIndividualData,
    benchmark: &'a crate::benchmark::BenchmarkReport,
}

pub fn write_benchmark_report(
    path: &Path,
    training_seed: u64,
    selector: &CandidateSelector,
    candidate: &EvaluatedIndividual,
    report: &crate::benchmark::BenchmarkReport,
) -> Result<(), PersistenceError> {
    let selector = match selector {
        CandidateSelector::BestEver => StandaloneSelectorData::BestEver,
        CandidateSelector::Generation(human_generation) => StandaloneSelectorData::Generation {
            human_generation: *human_generation,
            stored_generation_index: human_generation - 1,
        },
    };
    write_json_atomically(
        path,
        &StandaloneBenchmarkDocument {
            format: "blocky-evolution-benchmark",
            version: 1,
            training_seed,
            selector,
            candidate: EvaluatedIndividualData::from(candidate),
            benchmark: report,
        },
    )
}

fn io_error(operation: &'static str, path: &Path, source: io::Error) -> PersistenceError {
    PersistenceError::Io {
        operation,
        path: path.to_owned(),
        source,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrainingConfigData {
    search_depth: usize,
    max_game_plies: usize,
    master_seed: u64,
    opening_min_plies: usize,
    opening_max_plies: usize,
    max_opening_attempts: usize,
}

impl From<&TrainingConfig> for TrainingConfigData {
    fn from(config: &TrainingConfig) -> Self {
        Self {
            search_depth: config.search_depth(),
            max_game_plies: config.max_game_plies(),
            master_seed: config.master_seed(),
            opening_min_plies: *config.opening_plies().start(),
            opening_max_plies: *config.opening_plies().end(),
            max_opening_attempts: config.max_opening_attempts(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvolutionConfigData {
    training: TrainingConfigData,
    generations: usize,
    population_size: usize,
    swiss_rounds: usize,
    elite_count: usize,
    parent_candidate_count: usize,
    gene_mutation_probability: f64,
    strong_mutation_probability: f64,
    mutation_step: f64,
    strong_mutation_step: f64,
    #[serde(default)]
    default_anchor_weight_percent: u8,
    #[serde(default)]
    default_anchor_opening_pairs: usize,
}

impl From<&EvolutionConfig> for EvolutionConfigData {
    fn from(config: &EvolutionConfig) -> Self {
        Self {
            training: TrainingConfigData::from(config.training()),
            generations: config.generations(),
            population_size: config.population_size(),
            swiss_rounds: config.swiss_rounds(),
            elite_count: config.elite_count(),
            parent_candidate_count: config.parent_candidate_count(),
            gene_mutation_probability: config.gene_mutation_probability(),
            strong_mutation_probability: config.strong_mutation_probability(),
            mutation_step: config.mutation_step(),
            strong_mutation_step: config.strong_mutation_step(),
            default_anchor_weight_percent: config.default_anchor().weight_percent(),
            default_anchor_opening_pairs: config.default_anchor().opening_pairs(),
        }
    }
}

impl TryFrom<EvolutionConfigData> for EvolutionConfig {
    type Error = PersistenceError;

    fn try_from(value: EvolutionConfigData) -> Result<Self, Self::Error> {
        let training = TrainingConfig::new(
            value.training.search_depth,
            value.training.max_game_plies,
            value.training.master_seed,
            value.training.opening_min_plies..=value.training.opening_max_plies,
            value.training.max_opening_attempts,
        )
        .map_err(|error| {
            PersistenceError::CorruptData(format!("invalid training config: {error}"))
        })?;
        let anchor = DefaultAnchorConfig::new(
            value.default_anchor_weight_percent,
            value.default_anchor_opening_pairs,
        )
        .map_err(|error| {
            PersistenceError::CorruptData(format!("invalid default anchor config: {error}"))
        })?;
        EvolutionConfig::new(
            training,
            value.generations,
            value.population_size,
            value.swiss_rounds,
            value.elite_count,
            value.parent_candidate_count,
            value.gene_mutation_probability,
            value.strong_mutation_probability,
            value.mutation_step,
            value.strong_mutation_step,
        )
        .map_err(|error| {
            PersistenceError::CorruptData(format!("invalid evolution config: {error}"))
        })?
        .with_default_anchor(anchor)
        .map_err(|error| {
            PersistenceError::CorruptData(format!("invalid default anchor config: {error}"))
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndividualData {
    id: u64,
    genes: [f64; GENE_COUNT],
    gene_bits: [u64; GENE_COUNT],
}

impl From<&Individual> for IndividualData {
    fn from(individual: &Individual) -> Self {
        Self {
            id: individual.id().0,
            genes: *individual.genome().genes(),
            gene_bits: individual.genome().genes().map(f64::to_bits),
        }
    }
}

impl TryFrom<IndividualData> for Individual {
    type Error = PersistenceError;

    fn try_from(value: IndividualData) -> Result<Self, Self::Error> {
        let genes = value.gene_bits.map(f64::from_bits);
        if genes.iter().zip(value.genes).any(|(exact, decimal)| {
            !decimal.is_finite() || (exact - decimal).abs() > f64::EPSILON * exact.abs().max(1.0)
        }) {
            return Err(PersistenceError::CorruptData(
                "genome decimal values and exact bits disagree".into(),
            ));
        }
        let genome = Genome::new(genes)
            .map_err(|error| PersistenceError::CorruptData(format!("invalid genome: {error}")))?;
        if genome.genes() != &genes {
            return Err(PersistenceError::CorruptData(
                "genome is not in canonical form".into(),
            ));
        }
        Ok(Self::new(IndividualId(value.id), genome))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum EvaluatedIndividualData {
    Current(CurrentEvaluatedIndividualData),
    Legacy(LegacyEvaluatedIndividualData),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentEvaluatedIndividualData {
    individual: IndividualData,
    selection_score: SelectionScoreData,
    self_play_score: ScoreComponentData,
    default_anchor_score: Option<ScoreComponentData>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyEvaluatedIndividualData {
    individual: IndividualData,
    fitness_half_points: u32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionScoreData {
    units: u32,
    maximum_units: u32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScoreComponentData {
    half_points: u32,
    available_half_points: u32,
}

impl From<&EvaluatedIndividual> for EvaluatedIndividualData {
    fn from(value: &EvaluatedIndividual) -> Self {
        let fitness = value.fitness_score();
        Self::Current(CurrentEvaluatedIndividualData {
            individual: IndividualData::from(value.individual()),
            selection_score: SelectionScoreData {
                units: fitness.selection_units().0,
                maximum_units: fitness.maximum_selection_units(),
            },
            self_play_score: ScoreComponentData::from(fitness.self_play()),
            default_anchor_score: fitness.default_anchor().map(ScoreComponentData::from),
        })
    }
}

impl From<ScoreComponent> for ScoreComponentData {
    fn from(value: ScoreComponent) -> Self {
        Self {
            half_points: value.half_points().0,
            available_half_points: value.available_half_points(),
        }
    }
}

impl From<ScoreComponentData> for ScoreComponent {
    fn from(value: ScoreComponentData) -> Self {
        Self::new(Score(value.half_points), value.available_half_points)
    }
}

impl TryFrom<EvaluatedIndividualData> for EvaluatedIndividual {
    type Error = PersistenceError;

    fn try_from(value: EvaluatedIndividualData) -> Result<Self, Self::Error> {
        match value {
            EvaluatedIndividualData::Current(value) => Ok(Self::with_fitness(
                value.individual.try_into()?,
                FitnessScore::new(
                    Score(value.selection_score.units),
                    value.selection_score.maximum_units,
                    value.self_play_score.into(),
                    value.default_anchor_score.map(Into::into),
                ),
            )),
            EvaluatedIndividualData::Legacy(value) => Ok(Self::new(
                value.individual.try_into()?,
                Score(value.fitness_half_points),
            )),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GenerationData {
    index: usize,
    ranked: Vec<EvaluatedIndividualData>,
}

impl From<&GenerationResult> for GenerationData {
    fn from(value: &GenerationResult) -> Self {
        Self {
            index: value.index(),
            ranked: value
                .ranked()
                .iter()
                .map(EvaluatedIndividualData::from)
                .collect(),
        }
    }
}

impl TryFrom<GenerationData> for GenerationResult {
    type Error = PersistenceError;

    fn try_from(value: GenerationData) -> Result<Self, Self::Error> {
        GenerationResult::new(
            value.index,
            value
                .ranked
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        )
        .map_err(state_error)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvolutionStateData {
    next_generation: usize,
    population: Vec<IndividualData>,
    generations: Vec<GenerationData>,
    best_ever: EvaluatedIndividualData,
    next_id: u64,
    rng_state: u64,
}

impl From<&EvolutionState> for EvolutionStateData {
    fn from(value: &EvolutionState) -> Self {
        Self {
            next_generation: value.next_generation(),
            population: value
                .population()
                .iter()
                .map(IndividualData::from)
                .collect(),
            generations: value
                .generations()
                .iter()
                .map(GenerationData::from)
                .collect(),
            best_ever: EvaluatedIndividualData::from(value.best_ever()),
            next_id: value.next_id(),
            rng_state: value.rng_state(),
        }
    }
}

impl TryFrom<EvolutionStateData> for EvolutionState {
    type Error = PersistenceError;

    fn try_from(value: EvolutionStateData) -> Result<Self, Self::Error> {
        EvolutionState::new(
            value.next_generation,
            value
                .population
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            value
                .generations
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            value.best_ever.try_into()?,
            value.next_id,
            value.rng_state,
        )
        .map_err(state_error)
    }
}

fn state_error(error: EvolutionStateError) -> PersistenceError {
    PersistenceError::CorruptData(format!("invalid evolution state: {error}"))
}

#[derive(Serialize)]
struct ExperimentReportDocument {
    format: String,
    version: u32,
    evolution_config: EvolutionConfigData,
    validation_config: ValidationConfigData,
    generations: Vec<GenerationData>,
    champion: EvaluatedIndividualData,
    validation: ValidationData,
}

#[derive(Serialize)]
struct ValidationConfigData {
    search_depths: Vec<usize>,
    opening_count: usize,
    max_game_plies: usize,
    master_seed: u64,
    opening_min_plies: usize,
    opening_max_plies: usize,
    max_opening_attempts: usize,
    minimum_margin_half_points: u32,
}

impl From<&ValidationConfig> for ValidationConfigData {
    fn from(config: &ValidationConfig) -> Self {
        Self {
            search_depths: config.search_depths().to_vec(),
            opening_count: config.opening_count(),
            max_game_plies: config.max_game_plies(),
            master_seed: config.master_seed(),
            opening_min_plies: *config.opening_plies().start(),
            opening_max_plies: *config.opening_plies().end(),
            max_opening_attempts: config.max_opening_attempts(),
            minimum_margin_half_points: config.minimum_margin_half_points(),
        }
    }
}

#[derive(Serialize)]
struct ValidationData {
    candidate_score_half_points: u32,
    reference_score_half_points: u32,
    accepted: bool,
    by_depth: Vec<DepthValidationData>,
}

#[derive(Serialize)]
struct StandaloneValidationDocument {
    format: String,
    version: u32,
    training_seed: u64,
    selector: StandaloneSelectorData,
    candidate: EvaluatedIndividualData,
    validation_config: ValidationConfigData,
    validation: ValidationData,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum StandaloneSelectorData {
    BestEver,
    Generation {
        human_generation: usize,
        stored_generation_index: usize,
    },
}

pub fn write_validation_report(
    path: &Path,
    training_seed: u64,
    selector: &CandidateSelector,
    candidate: &EvaluatedIndividual,
    report: &crate::validation::ValidationReport,
) -> Result<(), PersistenceError> {
    let selector = match selector {
        CandidateSelector::BestEver => StandaloneSelectorData::BestEver,
        CandidateSelector::Generation(human_generation) => StandaloneSelectorData::Generation {
            human_generation: *human_generation,
            stored_generation_index: human_generation - 1,
        },
    };
    let document = StandaloneValidationDocument {
        format: "blocky-evolution-validation".to_owned(),
        version: 1,
        training_seed,
        selector,
        candidate: EvaluatedIndividualData::from(candidate),
        validation_config: ValidationConfigData::from(&report.config),
        validation: ValidationData::from(report),
    };
    write_json_atomically(path, &document)
}

#[derive(Serialize)]
struct DepthValidationData {
    search_depth: usize,
    candidate_score_half_points: u32,
    reference_score_half_points: u32,
    accepted: bool,
    statistics: GameStatisticsData,
    openings: Vec<OpeningValidationData>,
}

#[derive(Serialize)]
struct OpeningValidationData {
    opening_id: u64,
    opening_seed: u64,
    candidate_score_half_points: u32,
    reference_score_half_points: u32,
    games: [GameObservationData; 2],
}

#[derive(Serialize)]
struct GameObservationData {
    outcome: &'static str,
    draw_reason: Option<&'static str>,
    plies: usize,
}

impl From<GameObservation> for GameObservationData {
    fn from(game: GameObservation) -> Self {
        let (outcome, draw_reason) = match game.outcome {
            GameOutcome::WhiteWin => ("white_win", None),
            GameOutcome::BlackWin => ("black_win", None),
            GameOutcome::Draw(reason) => (
                "draw",
                Some(match reason {
                    DrawReason::Stalemate => "stalemate",
                    DrawReason::InsufficientMaterial => "insufficient_material",
                    DrawReason::ThreefoldRepetition => "threefold_repetition",
                    DrawReason::FiftyMoveRule => "fifty_move_rule",
                    DrawReason::MaxPlies => "max_plies",
                }),
            ),
        };
        Self {
            outcome,
            draw_reason,
            plies: game.plies,
        }
    }
}

#[derive(Serialize)]
struct GameStatisticsData {
    games: usize,
    white_wins: usize,
    black_wins: usize,
    draws: usize,
    stalemates: usize,
    insufficient_material: usize,
    threefold_repetitions: usize,
    fifty_move_rule: usize,
    max_plies_draws: usize,
    total_plies: usize,
    mean_plies: f64,
    minimum_plies: usize,
    median_plies: usize,
    p95_plies: usize,
    maximum_plies: usize,
}

impl From<GameStatistics> for GameStatisticsData {
    fn from(statistics: GameStatistics) -> Self {
        Self {
            games: statistics.games,
            white_wins: statistics.white_wins,
            black_wins: statistics.black_wins,
            draws: statistics.draws,
            stalemates: statistics.stalemates,
            insufficient_material: statistics.insufficient_material,
            threefold_repetitions: statistics.threefold_repetitions,
            fifty_move_rule: statistics.fifty_move_rule,
            max_plies_draws: statistics.max_plies_draws,
            total_plies: statistics.total_plies,
            mean_plies: statistics.mean_plies(),
            minimum_plies: statistics.minimum_plies,
            median_plies: statistics.median_plies,
            p95_plies: statistics.p95_plies,
            maximum_plies: statistics.maximum_plies,
        }
    }
}

impl From<&ExperimentReport> for ValidationData {
    fn from(report: &ExperimentReport) -> Self {
        Self::from(report.validation())
    }
}

impl From<&crate::validation::ValidationReport> for ValidationData {
    fn from(validation: &crate::validation::ValidationReport) -> Self {
        Self {
            candidate_score_half_points: validation.candidate_score.0,
            reference_score_half_points: validation.reference_score.0,
            accepted: validation.accepted,
            by_depth: validation
                .by_depth
                .iter()
                .map(|depth| DepthValidationData {
                    search_depth: depth.search_depth,
                    candidate_score_half_points: depth.candidate_score.0,
                    reference_score_half_points: depth.reference_score.0,
                    accepted: depth.accepted,
                    statistics: GameStatisticsData::from(GameStatistics::from_observations(
                        depth.openings.iter().flat_map(|opening| opening.games),
                    )),
                    openings: depth
                        .openings
                        .iter()
                        .map(|opening| OpeningValidationData {
                            opening_id: opening.opening.0,
                            opening_seed: opening.opening_seed,
                            candidate_score_half_points: opening.candidate_score.0,
                            reference_score_half_points: opening.reference_score.0,
                            games: opening.games.map(GameObservationData::from),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        evolution::EvolutionResult,
        validation::{DepthValidationResult, OpeningValidationResult, ValidationReport},
    };

    fn path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "blocky-evolution-{name}-{}.json",
            std::process::id()
        ))
    }

    fn individual(id: u64, value: f64) -> Individual {
        let mut genes = [value; GENE_COUNT];
        genes[0] = 1.0;
        Individual::new(IndividualId(id), Genome::new(genes).unwrap())
    }

    fn state() -> EvolutionState {
        let population: Vec<_> = (0..4)
            .map(|id| individual(id, 0.1 + id as f64 / 10.0))
            .collect();
        let ranked: Vec<_> = population
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, individual)| {
                EvaluatedIndividual::new(individual, Score(4 - index as u32))
            })
            .collect();
        let generation = GenerationResult::new(0, ranked.clone()).unwrap();
        EvolutionState::new(1, population, vec![generation], ranked[0].clone(), 4, 99).unwrap()
    }

    fn anchored_state() -> EvolutionState {
        let population: Vec<_> = (0..4)
            .map(|id| individual(id, 0.1 + id as f64 / 10.0))
            .collect();
        let ranked = population
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, individual)| {
                let self_play = Score(4 - index as u32);
                let anchor = Score(index as u32);
                EvaluatedIndividual::with_fitness(
                    individual,
                    FitnessScore::new(
                        crate::evolution::anchored_selection_score(
                            self_play,
                            anchor,
                            1,
                            DefaultAnchorConfig::new(10, 1).unwrap(),
                        ),
                        400,
                        ScoreComponent::new(self_play, 4),
                        Some(ScoreComponent::new(anchor, 4)),
                    ),
                )
            })
            .collect::<Vec<_>>();
        let generation = GenerationResult::new(0, ranked.clone()).unwrap();
        EvolutionState::new(1, population, vec![generation], ranked[0].clone(), 4, 99).unwrap()
    }

    fn config() -> EvolutionConfig {
        let defaults = EvolutionConfig::default();
        EvolutionConfig::new(
            defaults.training().clone(),
            3,
            4,
            1,
            1,
            2,
            defaults.gene_mutation_probability(),
            defaults.strong_mutation_probability(),
            defaults.mutation_step(),
            defaults.strong_mutation_step(),
        )
        .unwrap()
    }

    #[test]
    fn checkpoint_round_trip_preserves_exact_resumable_state_and_header() {
        let output = path("checkpoint-round-trip");
        let expected = state();

        write_checkpoint(&output, &config(), &expected).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(json["format"], PERSISTENCE_FORMAT);
        assert_eq!(json["version"], PERSISTENCE_VERSION);
        assert_eq!(read_checkpoint(&output, &config()).unwrap(), expected);

        fs::remove_file(output).unwrap();
    }

    #[test]
    fn anchored_checkpoint_uses_auditable_score_fields_and_round_trips_components() {
        let output = path("anchored-score-round-trip");
        let config = config()
            .with_default_anchor(DefaultAnchorConfig::new(10, 1).unwrap())
            .unwrap();
        let expected = anchored_state();
        write_checkpoint(&output, &config, &expected).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        let first = &json["state"]["generations"][0]["ranked"][0];
        assert!(first.get("fitness_half_points").is_none());
        assert_eq!(first["selection_score"]["maximum_units"], 400);
        assert_eq!(first["self_play_score"]["available_half_points"], 4);
        assert_eq!(first["default_anchor_score"]["available_half_points"], 4);
        assert_eq!(read_checkpoint(&output, &config).unwrap(), expected);

        let mut corrupt = json;
        corrupt["state"]["generations"][0]["ranked"][0]["selection_score"]["units"] = 999.into();
        fs::write(&output, serde_json::to_vec(&corrupt).unwrap()).unwrap();
        assert!(matches!(
            read_checkpoint(&output, &config),
            Err(PersistenceError::CorruptData(message))
                if message.contains("selection score")
        ));
        fs::remove_file(output).unwrap();
    }

    #[test]
    fn legacy_zero_anchor_checkpoint_remains_readable_and_anchor_mismatch_is_rejected() {
        let output = path("legacy-zero-anchor");
        let expected = state();
        write_checkpoint(&output, &config(), &expected).unwrap();
        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        let evolution = json["evolution_config"].as_object_mut().unwrap();
        evolution.remove("default_anchor_weight_percent");
        evolution.remove("default_anchor_opening_pairs");
        json["version"] = LEGACY_PERSISTENCE_VERSION.into();
        for ranked in json["state"]["generations"].as_array_mut().unwrap() {
            for evaluated in ranked["ranked"].as_array_mut().unwrap() {
                migrate_to_legacy_evaluated_json(evaluated);
            }
        }
        migrate_to_legacy_evaluated_json(&mut json["state"]["best_ever"]);
        fs::write(&output, serde_json::to_vec(&json).unwrap()).unwrap();
        assert_eq!(read_checkpoint(&output, &config()).unwrap(), expected);

        let anchored = config()
            .with_default_anchor(DefaultAnchorConfig::new(10, 1).unwrap())
            .unwrap();
        assert!(matches!(
            read_checkpoint(&output, &anchored),
            Err(PersistenceError::IncompatibleEvolutionConfig)
        ));
        fs::remove_file(output).unwrap();
    }

    fn migrate_to_legacy_evaluated_json(value: &mut serde_json::Value) {
        let object = value.as_object_mut().unwrap();
        let units = object["selection_score"]["units"].as_u64().unwrap();
        object.remove("selection_score");
        object.remove("self_play_score");
        object.remove("default_anchor_score");
        object.insert("fitness_half_points".into(), units.into());
    }

    #[test]
    fn checkpoint_rejects_version_config_and_corruption_with_typed_errors() {
        let output = path("checkpoint-errors");
        write_checkpoint(&output, &config(), &state()).unwrap();
        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        json["version"] = 999.into();
        fs::write(&output, serde_json::to_vec(&json).unwrap()).unwrap();
        assert!(matches!(
            read_checkpoint(&output, &config()),
            Err(PersistenceError::UnsupportedVersion(999))
        ));

        write_checkpoint(&output, &config(), &state()).unwrap();
        assert!(matches!(
            read_checkpoint(&output, &EvolutionConfig::default()),
            Err(PersistenceError::IncompatibleEvolutionConfig)
        ));

        fs::write(&output, b"{ definitely not JSON").unwrap();
        assert!(matches!(
            read_checkpoint(&output, &config()),
            Err(PersistenceError::InvalidJson(_))
        ));
        fs::remove_file(output).unwrap();
    }

    #[test]
    fn exported_report_contains_configs_seeds_history_champion_and_validation() {
        let output = path("report");
        let state = state();
        let evolution =
            EvolutionResult::new(state.generations().to_vec(), state.best_ever().clone()).unwrap();
        let validation_config = ValidationConfig::default();
        let opening = OpeningValidationResult {
            opening: crate::openings::OpeningId(7),
            opening_seed: 123,
            candidate_score: Score(3),
            reference_score: Score(1),
            games: [
                GameObservation {
                    outcome: GameOutcome::WhiteWin,
                    plies: 21,
                },
                GameObservation {
                    outcome: GameOutcome::Draw(DrawReason::ThreefoldRepetition),
                    plies: 42,
                },
            ],
        };
        let validation = ValidationReport {
            config: validation_config,
            by_depth: vec![DepthValidationResult {
                search_depth: 4,
                candidate_score: Score(3),
                reference_score: Score(1),
                accepted: true,
                openings: vec![opening],
            }],
            candidate_score: Score(3),
            reference_score: Score(1),
            accepted: true,
        };
        let report = ExperimentReport::new(evolution, validation);

        write_experiment_report(&output, &config(), &report).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(json["evolution_config"]["training"]["master_seed"], 0);
        assert_eq!(
            json["validation_config"]["master_seed"],
            ValidationConfig::default().master_seed()
        );
        assert_eq!(json["generations"].as_array().unwrap().len(), 1);
        assert_eq!(json["champion"]["individual"]["id"], 0);
        assert_eq!(
            json["validation"]["by_depth"][0]["openings"][0]["opening_seed"],
            123
        );
        assert_eq!(json["validation"]["by_depth"][0]["statistics"]["games"], 2);
        assert_eq!(
            json["validation"]["by_depth"][0]["openings"][0]["games"][1]["draw_reason"],
            "threefold_repetition"
        );

        fs::remove_file(output).unwrap();
    }
}
