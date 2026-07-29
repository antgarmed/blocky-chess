//! Deterministic, observational comparison of evolved checkpoint individuals.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, fs,
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    benchmark::{SerializableObservation, SerializableStatistics},
    encounter::{GameRunner, GameRunnerFactory},
    evolution::Individual,
    historical::phenotype_fingerprint,
    openings::{Opening, OpeningGenerationError, OpeningPool},
    persistence::{read_checkpoint_unchecked_config, write_json_atomically, PersistenceError},
    rng::derive_seed,
    self_play::GameOutcome,
    telemetry::{GameObservation, GameStatistics},
    training::TrainingConfig,
    validation::ValidationConfig,
    GENE_COUNT,
};

pub const RETENTION_FORMAT: &str = "blocky-evolution-retention-benchmark";
pub const RETENTION_VERSION: u32 = 1;
const OPPONENT_OPENING_SEED_DOMAIN: u64 = 0x5245_5445_4e54_4f50;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionManifest {
    pub config: RetentionConfig,
    pub candidates: Vec<CheckpointSelection>,
    pub opponents: Vec<CheckpointSelection>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionConfig {
    pub search_depth: usize,
    pub opening_pairs_per_opponent: usize,
    pub max_game_plies: usize,
    pub opening_min_plies: usize,
    pub opening_max_plies: usize,
    pub max_opening_attempts: usize,
    pub seed: u64,
    pub workers: usize,
}

impl RetentionConfig {
    pub fn opening_plies(&self) -> RangeInclusive<usize> {
        self.opening_min_plies..=self.opening_max_plies
    }

    pub fn validate(&self) -> Result<(), RetentionError> {
        let checks = [
            (self.search_depth > 0, "search depth must be positive"),
            (
                self.opening_pairs_per_opponent > 0,
                "opening-pair count must be positive",
            ),
            (
                self.max_game_plies > 0,
                "maximum game plies must be positive",
            ),
            (
                self.opening_min_plies <= self.opening_max_plies,
                "opening minimum must not exceed maximum",
            ),
            (
                self.max_opening_attempts > 0,
                "maximum opening attempts must be positive",
            ),
            (self.workers > 0, "worker count must be positive"),
        ];
        if let Some((_, message)) = checks.into_iter().find(|(valid, _)| !valid) {
            return Err(RetentionError::InvalidConfig(message.into()));
        }
        TrainingConfig::new(
            self.search_depth,
            self.max_game_plies,
            self.seed,
            self.opening_plies(),
            self.max_opening_attempts,
        )
        .map_err(|error| RetentionError::InvalidConfig(error.to_string()))?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointSelection {
    pub label: String,
    pub checkpoint: PathBuf,
    pub generation: GenerationSelector,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum GenerationSelector {
    Generation { human_generation: usize },
    BestEver,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedIndividual {
    pub label: String,
    pub checkpoint: PathBuf,
    pub selector: GenerationSelector,
    pub resolved_generation: usize,
    pub training_seed: u64,
    pub checkpoint_sha256: String,
    pub individual: Individual,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RetentionReport {
    pub format: String,
    pub version: u32,
    pub configuration: RetentionConfig,
    pub opening_derivation: OpeningDerivation,
    pub candidates: Vec<ResolvedIndividualReport>,
    pub opponents: Vec<ResolvedIndividualReport>,
    pub opponent_openings: Vec<OpponentOpeningPool>,
    pub total_opening_positions: usize,
    pub candidate_results: Vec<CandidateResult>,
    pub total_games: usize,
    pub statistics: SerializableStatistics,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpeningDerivation {
    pub algorithm: String,
    pub master_seed: u64,
    pub candidate_independent: bool,
    pub candidate_order_independent: bool,
    pub opponent_specific: bool,
    pub opponent_order_independent: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedIndividualReport {
    pub label: String,
    pub checkpoint_source: PathBuf,
    pub generation_selector: GenerationSelector,
    pub resolved_generation: usize,
    pub training_seed: u64,
    pub checkpoint_sha256: String,
    pub individual_id: u64,
    pub genome: [f64; GENE_COUNT],
    pub effective_phenotype_fingerprint: [i64; 13],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedOpening {
    pub opening_id: u64,
    pub opening_seed: u64,
    pub opening_plies: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OpponentOpeningPool {
    pub opponent_label: String,
    pub opponent_identity_seed: u64,
    pub opening_master_seed: u64,
    pub openings: Vec<SharedOpening>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CandidateResult {
    pub candidate_label: String,
    pub score_half_points: u32,
    pub available_half_points: u32,
    pub wins: usize,
    pub draws: usize,
    pub losses: usize,
    pub by_color: ColorSplit,
    pub statistics: SerializableStatistics,
    pub versus: Vec<PairingResult>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ColorResult {
    pub games: usize,
    pub score_half_points: u32,
    pub available_half_points: u32,
    pub wins: usize,
    pub draws: usize,
    pub losses: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ColorSplit {
    pub as_white: ColorResult,
    pub as_black: ColorResult,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PairingResult {
    pub opponent_label: String,
    pub score_half_points: u32,
    pub available_half_points: u32,
    pub wins: usize,
    pub draws: usize,
    pub losses: usize,
    pub by_color: ColorSplit,
    pub statistics: SerializableStatistics,
    pub opening_results: Vec<OpeningPairResult>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OpeningPairResult {
    pub opening_id: u64,
    pub opening_seed: u64,
    pub candidate_score_half_points: u32,
    pub available_half_points: u32,
    pub candidate_as_white: SerializableObservation,
    pub candidate_as_black: SerializableObservation,
}

#[derive(Debug)]
pub enum RetentionError {
    ReadManifest {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidManifest(serde_json::Error),
    InvalidConfig(String),
    EmptyCandidates,
    EmptyOpponents,
    ResolvedSetMismatch,
    DuplicateLabel(String),
    InvalidLabel,
    ZeroGeneration(String),
    Checkpoint {
        label: String,
        source: PersistenceError,
    },
    UnavailableGeneration {
        label: String,
        requested: usize,
        available: usize,
    },
    SeedCollision {
        seed: u64,
        labels: Vec<String>,
    },
    DuplicateCandidatePhenotype {
        first: String,
        second: String,
    },
    DuplicateOpponentPhenotype {
        first: String,
        second: String,
    },
    DuplicateResolvedIndividual {
        first: String,
        second: String,
    },
    DistinctOpponentOpeningsExhausted {
        label: String,
    },
    Opening(OpeningGenerationError),
    Game(String),
    WorkerPanic,
    Persistence(PersistenceError),
}

impl fmt::Display for RetentionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadManifest { path, source } => write!(f, "could not read manifest {}: {source}", path.display()),
            Self::InvalidManifest(source) => write!(f, "invalid retention manifest JSON: {source}"),
            Self::InvalidConfig(message) => write!(f, "invalid retention configuration: {message}"),
            Self::EmptyCandidates => f.write_str("retention manifest has no candidates"),
            Self::EmptyOpponents => f.write_str("retention manifest has no opponents"),
            Self::ResolvedSetMismatch => {
                f.write_str("resolved candidates or opponents do not match the manifest")
            }
            Self::DuplicateLabel(label) => write!(f, "duplicate retention label `{label}`"),
            Self::InvalidLabel => f.write_str("retention labels must not be empty"),
            Self::ZeroGeneration(label) => write!(f, "generation for `{label}` is 1-based and must be positive"),
            Self::Checkpoint { label, source } => write!(f, "could not resolve `{label}` checkpoint: {source}"),
            Self::UnavailableGeneration { label, requested, available } => write!(f, "generation {requested} for `{label}` is unavailable; checkpoint contains {available} completed generations"),
            Self::SeedCollision { seed, labels } => write!(f, "retention seed {seed} collides with {}", labels.join(", ")),
            Self::DuplicateCandidatePhenotype { first, second } => write!(f, "candidate labels `{first}` and `{second}` resolve to the same effective phenotype"),
            Self::DuplicateOpponentPhenotype { first, second } => write!(f, "opponent labels `{first}` and `{second}` resolve to the same effective phenotype"),
            Self::DuplicateResolvedIndividual { first, second } => write!(f, "labels `{first}` and `{second}` resolve to the same individual"),
            Self::DistinctOpponentOpeningsExhausted { label } => write!(f, "could not derive a distinct opening pool for opponent `{label}`"),
            Self::Opening(source) => write!(f, "could not generate opponent opening pool: {source}"),
            Self::Game(message) => write!(f, "retention game failed: {message}"),
            Self::WorkerPanic => f.write_str("retention worker panicked"),
            Self::Persistence(source) => write!(f, "{source}"),
        }
    }
}

impl Error for RetentionError {}

pub fn read_manifest(path: &Path) -> Result<RetentionManifest, RetentionError> {
    let bytes = fs::read(path).map_err(|source| RetentionError::ReadManifest {
        path: path.to_owned(),
        source,
    })?;
    let mut manifest: RetentionManifest =
        serde_json::from_slice(&bytes).map_err(RetentionError::InvalidManifest)?;
    let base = path.parent().unwrap_or(Path::new("."));
    for selection in manifest
        .candidates
        .iter_mut()
        .chain(manifest.opponents.iter_mut())
    {
        if selection.checkpoint.is_relative() {
            selection.checkpoint = base.join(&selection.checkpoint);
        }
    }
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn validate_manifest(manifest: &RetentionManifest) -> Result<(), RetentionError> {
    manifest.config.validate()?;
    if manifest.candidates.is_empty() {
        return Err(RetentionError::EmptyCandidates);
    }
    if manifest.opponents.is_empty() {
        return Err(RetentionError::EmptyOpponents);
    }
    let mut labels = BTreeSet::new();
    for item in manifest.candidates.iter().chain(&manifest.opponents) {
        if item.label.trim().is_empty() {
            return Err(RetentionError::InvalidLabel);
        }
        if !labels.insert(item.label.clone()) {
            return Err(RetentionError::DuplicateLabel(item.label.clone()));
        }
        if matches!(
            item.generation,
            GenerationSelector::Generation {
                human_generation: 0
            }
        ) {
            return Err(RetentionError::ZeroGeneration(item.label.clone()));
        }
    }
    Ok(())
}

pub fn resolve_selections(
    selections: &[CheckpointSelection],
) -> Result<Vec<ResolvedIndividual>, RetentionError> {
    let mut cache = BTreeMap::new();
    resolve_with_cache(selections, &mut cache)
}

pub fn resolve_manifest(
    manifest: &RetentionManifest,
) -> Result<(Vec<ResolvedIndividual>, Vec<ResolvedIndividual>), RetentionError> {
    validate_manifest(manifest)?;
    let mut cache = BTreeMap::new();
    let candidates = resolve_with_cache(&manifest.candidates, &mut cache)?;
    let opponents = resolve_with_cache(&manifest.opponents, &mut cache)?;
    validate_resolved(manifest.config.seed, &candidates, &opponents)?;
    Ok((candidates, opponents))
}

fn resolve_with_cache(
    selections: &[CheckpointSelection],
    cache: &mut BTreeMap<PathBuf, CheckpointData>,
) -> Result<Vec<ResolvedIndividual>, RetentionError> {
    selections
        .iter()
        .map(|selection| {
            if matches!(
                selection.generation,
                GenerationSelector::Generation {
                    human_generation: 0
                }
            ) {
                return Err(RetentionError::ZeroGeneration(selection.label.clone()));
            }
            let checkpoint = if let Some(value) = cache.get(&selection.checkpoint) {
                value
            } else {
                let bytes = fs::read(&selection.checkpoint).map_err(|source| {
                    RetentionError::Checkpoint {
                        label: selection.label.clone(),
                        source: PersistenceError::Io {
                            operation: "read",
                            path: selection.checkpoint.clone(),
                            source,
                        },
                    }
                })?;
                let (config, state) = read_checkpoint_unchecked_config(&selection.checkpoint)
                    .map_err(|source| RetentionError::Checkpoint {
                        label: selection.label.clone(),
                        source,
                    })?;
                cache.insert(
                    selection.checkpoint.clone(),
                    CheckpointData {
                        training_seed: config.training().master_seed(),
                        state,
                        sha256: hex_digest(&bytes),
                    },
                );
                cache
                    .get(&selection.checkpoint)
                    .expect("inserted checkpoint")
            };
            let (resolved_generation, individual) = match selection.generation {
                GenerationSelector::Generation { human_generation } => {
                    let generation = checkpoint
                        .state
                        .generations()
                        .get(human_generation - 1)
                        .ok_or(RetentionError::UnavailableGeneration {
                            label: selection.label.clone(),
                            requested: human_generation,
                            available: checkpoint.state.generations().len(),
                        })?;
                    (human_generation, generation.best().individual().clone())
                }
                GenerationSelector::BestEver => {
                    let best = checkpoint.state.best_ever();
                    let generation = checkpoint
                        .state
                        .generations()
                        .iter()
                        .position(|entry| entry.ranked().contains(best))
                        .map_or(0, |index| index + 1);
                    (generation, best.individual().clone())
                }
            };
            Ok(ResolvedIndividual {
                label: selection.label.clone(),
                checkpoint: selection.checkpoint.clone(),
                selector: selection.generation.clone(),
                resolved_generation,
                training_seed: checkpoint.training_seed,
                checkpoint_sha256: checkpoint.sha256.clone(),
                individual,
            })
        })
        .collect()
}

struct CheckpointData {
    training_seed: u64,
    state: crate::evolution::EvolutionState,
    sha256: String,
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_resolved(
    retention_seed: u64,
    candidates: &[ResolvedIndividual],
    opponents: &[ResolvedIndividual],
) -> Result<(), RetentionError> {
    let validation_seed = ValidationConfig::default().master_seed();
    let labels = candidates
        .iter()
        .chain(opponents)
        .filter(|individual| individual.training_seed == retention_seed)
        .map(|individual| format!("{} training seed", individual.label))
        .chain((validation_seed == retention_seed).then(|| "default final-validation seed".into()))
        .collect::<Vec<_>>();
    if !labels.is_empty() {
        return Err(RetentionError::SeedCollision {
            seed: retention_seed,
            labels,
        });
    }
    check_duplicate_phenotypes(candidates, true)?;
    check_duplicate_phenotypes(opponents, false)?;
    let mut identities = BTreeMap::new();
    for individual in candidates.iter().chain(opponents) {
        let identity = (
            individual.checkpoint_sha256.clone(),
            individual.resolved_generation,
            individual.individual.id().0,
        );
        if let Some(first) = identities.insert(identity, individual.label.clone()) {
            return Err(RetentionError::DuplicateResolvedIndividual {
                first,
                second: individual.label.clone(),
            });
        }
    }
    Ok(())
}

fn check_duplicate_phenotypes(
    individuals: &[ResolvedIndividual],
    candidate: bool,
) -> Result<(), RetentionError> {
    let mut fingerprints = BTreeMap::new();
    for individual in individuals {
        let fingerprint = phenotype_fingerprint(individual.individual.genome());
        if let Some(first) = fingerprints.insert(fingerprint, individual.label.clone()) {
            return Err(if candidate {
                RetentionError::DuplicateCandidatePhenotype {
                    first,
                    second: individual.label.clone(),
                }
            } else {
                RetentionError::DuplicateOpponentPhenotype {
                    first,
                    second: individual.label.clone(),
                }
            });
        }
    }
    Ok(())
}

pub fn run_retention<F>(
    manifest: &RetentionManifest,
    candidates: &[ResolvedIndividual],
    opponents: &[ResolvedIndividual],
    factory: F,
) -> Result<RetentionReport, RetentionError>
where
    F: GameRunnerFactory + Sync,
    F::Runner: Send,
    <F::Runner as GameRunner>::Error: fmt::Display,
{
    validate_manifest(manifest)?;
    if candidates.len() != manifest.candidates.len()
        || opponents.len() != manifest.opponents.len()
        || candidates
            .iter()
            .zip(&manifest.candidates)
            .any(|(resolved, requested)| resolved.label != requested.label)
        || opponents
            .iter()
            .zip(&manifest.opponents)
            .any(|(resolved, requested)| resolved.label != requested.label)
    {
        return Err(RetentionError::ResolvedSetMismatch);
    }
    validate_resolved(manifest.config.seed, candidates, opponents)?;
    let pools = generate_opponent_pools(opponents, &manifest.config)?;
    let pool_indices = opponents
        .iter()
        .map(|opponent| {
            pools
                .iter()
                .position(|pool| pool.opponent_label == opponent.label)
                .expect("every opponent has an opening pool")
        })
        .collect::<Vec<_>>();
    let mut tasks = Vec::new();
    for candidate_index in 0..candidates.len() {
        for opponent_index in 0..opponents.len() {
            for opening_index in 0..pools[pool_indices[opponent_index]].openings.len() {
                tasks.push((candidate_index, opponent_index, opening_index));
            }
        }
    }
    let worker_count = manifest.config.workers.min(tasks.len());
    let chunks = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for worker in 0..worker_count {
            let factory = &factory;
            let tasks = &tasks;
            let pools = &pools;
            let pool_indices = &pool_indices;
            handles.push(scope.spawn(move || {
                let mut runner = factory.create();
                tasks
                    .iter()
                    .copied()
                    .enumerate()
                    .skip(worker)
                    .step_by(worker_count)
                    .map(|(task_index, (candidate, opponent, opening))| {
                        let result = play_pair(
                            &mut runner,
                            &candidates[candidate].individual,
                            &opponents[opponent].individual,
                            &pools[pool_indices[opponent]].openings[opening],
                            &manifest.config,
                        );
                        (task_index, result)
                    })
                    .collect::<Vec<_>>()
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join())
            .collect::<Vec<_>>()
    });
    let mut ordered = vec![None; tasks.len()];
    for chunk in chunks {
        for (index, result) in chunk.map_err(|_| RetentionError::WorkerPanic)? {
            ordered[index] = Some(result?);
        }
    }
    let pairs = ordered.into_iter().map(Option::unwrap).collect::<Vec<_>>();
    Ok(build_report(
        manifest, candidates, opponents, &pools, &pairs,
    ))
}

fn generate_opponent_pools(
    opponents: &[ResolvedIndividual],
    config: &RetentionConfig,
) -> Result<Vec<GeneratedOpponentPool>, RetentionError> {
    let mut ordered = opponents.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|opponent| {
        (
            opponent_identity_seed(opponent),
            opponent.checkpoint_sha256.clone(),
            opponent.resolved_generation,
            opponent.individual.id().0,
        )
    });
    let mut pools = Vec::with_capacity(ordered.len());
    for opponent in ordered {
        let pool = generate_opponent_pool(opponent, config, &pools)?;
        pools.push(pool);
    }
    Ok(pools)
}

fn generate_opponent_pool(
    opponent: &ResolvedIndividual,
    config: &RetentionConfig,
    existing: &[GeneratedOpponentPool],
) -> Result<GeneratedOpponentPool, RetentionError> {
    let identity_seed = opponent_identity_seed(opponent);
    for retry in 0..config.max_opening_attempts {
        let seed = derive_seed(
            config.seed,
            identity_seed,
            OPPONENT_OPENING_SEED_DOMAIN.wrapping_add(retry as u64),
        );
        let opening_config = TrainingConfig::new(
            config.search_depth,
            config.max_game_plies,
            seed,
            config.opening_plies(),
            config.max_opening_attempts,
        )
        .map_err(|error| RetentionError::InvalidConfig(error.to_string()))?;
        let pool = OpeningPool::generate(config.opening_pairs_per_opponent, &opening_config)
            .map_err(RetentionError::Opening)?;
        let openings = pool.openings().to_vec();
        if openings.iter().all(|opening| {
            existing.iter().all(|other| {
                other
                    .openings
                    .iter()
                    .all(|known| known.position != opening.position)
            })
        }) {
            return Ok(GeneratedOpponentPool {
                opponent_label: opponent.label.clone(),
                identity_seed,
                opening_master_seed: seed,
                openings,
            });
        }
    }
    Err(RetentionError::DistinctOpponentOpeningsExhausted {
        label: opponent.label.clone(),
    })
}

#[derive(Clone)]
struct GeneratedOpponentPool {
    opponent_label: String,
    identity_seed: u64,
    opening_master_seed: u64,
    openings: Vec<Opening>,
}

fn opponent_identity_seed(opponent: &ResolvedIndividual) -> u64 {
    let mut stream = 0;
    for chunk in opponent.checkpoint_sha256.as_bytes().chunks(8) {
        let mut word = [0_u8; 8];
        word[..chunk.len()].copy_from_slice(chunk);
        stream = derive_seed(
            stream,
            u64::from_le_bytes(word),
            OPPONENT_OPENING_SEED_DOMAIN,
        );
    }
    stream = derive_seed(stream, opponent.resolved_generation as u64, 1);
    stream = derive_seed(stream, opponent.individual.id().0, 2);
    for (index, value) in phenotype_fingerprint(opponent.individual.genome())
        .into_iter()
        .enumerate()
    {
        stream = derive_seed(stream, value as u64, 3 + index as u64);
    }
    stream
}

fn play_pair<R: GameRunner>(
    runner: &mut R,
    candidate: &Individual,
    opponent: &Individual,
    opening: &Opening,
    config: &RetentionConfig,
) -> Result<OpeningPairResult, RetentionError>
where
    R::Error: fmt::Display,
{
    let white = runner
        .play(
            candidate.genome(),
            opponent.genome(),
            opening,
            config.search_depth,
            config.max_game_plies,
        )
        .map_err(|error| RetentionError::Game(error.to_string()))?;
    let black = runner
        .play(
            opponent.genome(),
            candidate.genome(),
            opening,
            config.search_depth,
            config.max_game_plies,
        )
        .map_err(|error| RetentionError::Game(error.to_string()))?;
    let score = score_as_white(white.outcome) + score_as_black(black.outcome);
    Ok(OpeningPairResult {
        opening_id: opening.id.0,
        opening_seed: opening.seed,
        candidate_score_half_points: score,
        available_half_points: 4,
        candidate_as_white: GameObservation::from(&white).into(),
        candidate_as_black: GameObservation::from(&black).into(),
    })
}

fn build_report(
    manifest: &RetentionManifest,
    candidates: &[ResolvedIndividual],
    opponents: &[ResolvedIndividual],
    pools: &[GeneratedOpponentPool],
    pairs: &[OpeningPairResult],
) -> RetentionReport {
    let per_pairing = manifest.config.opening_pairs_per_opponent;
    let per_candidate = opponents.len() * per_pairing;
    let mut candidate_results = Vec::new();
    for (candidate_index, candidate) in candidates.iter().enumerate() {
        let mut versus = Vec::new();
        for (opponent_index, opponent) in opponents.iter().enumerate() {
            let start = candidate_index * per_candidate + opponent_index * per_pairing;
            versus.push(summarize_pairing(
                opponent.label.clone(),
                pairs[start..start + per_pairing].to_vec(),
            ));
        }
        candidate_results.push(summarize_candidate(candidate.label.clone(), versus));
    }
    let all_observations = pairs.iter().flat_map(pair_observations);
    RetentionReport {
        format: RETENTION_FORMAT.into(),
        version: RETENTION_VERSION,
        configuration: manifest.config.clone(),
        opening_derivation: OpeningDerivation {
            algorithm: "OpeningPool::generate(seed, opening-index, attempt)".into(),
            master_seed: manifest.config.seed,
            candidate_independent: true,
            candidate_order_independent: true,
            opponent_specific: true,
            opponent_order_independent: true,
        },
        candidates: candidates.iter().map(resolved_report).collect(),
        opponents: opponents.iter().map(resolved_report).collect(),
        opponent_openings: pools
            .iter()
            .map(|pool| OpponentOpeningPool {
                opponent_label: pool.opponent_label.clone(),
                opponent_identity_seed: pool.identity_seed,
                opening_master_seed: pool.opening_master_seed,
                openings: pool.openings.iter().map(shared_opening).collect(),
            })
            .collect(),
        total_opening_positions: pools.iter().map(|pool| pool.openings.len()).sum(),
        total_games: pairs.len() * 2,
        statistics: GameStatistics::from_observations(all_observations).into(),
        candidate_results,
    }
}

fn shared_opening(opening: &Opening) -> SharedOpening {
    SharedOpening {
        opening_id: opening.id.0,
        opening_seed: opening.seed,
        opening_plies: opening.moves.len(),
    }
}

fn resolved_report(value: &ResolvedIndividual) -> ResolvedIndividualReport {
    ResolvedIndividualReport {
        label: value.label.clone(),
        checkpoint_source: value.checkpoint.clone(),
        generation_selector: value.selector.clone(),
        resolved_generation: value.resolved_generation,
        training_seed: value.training_seed,
        checkpoint_sha256: value.checkpoint_sha256.clone(),
        individual_id: value.individual.id().0,
        genome: *value.individual.genome().genes(),
        effective_phenotype_fingerprint: phenotype_fingerprint(value.individual.genome()),
    }
}

fn summarize_pairing(label: String, opening_results: Vec<OpeningPairResult>) -> PairingResult {
    let mut color = ColorSplit::default();
    let mut observations = Vec::new();
    for pair in &opening_results {
        record_color(&mut color.as_white, pair.candidate_as_white, true);
        record_color(&mut color.as_black, pair.candidate_as_black, false);
        observations.extend(pair_observations(pair));
    }
    let (wins, draws, losses) = totals(&color);
    PairingResult {
        opponent_label: label,
        score_half_points: color.as_white.score_half_points + color.as_black.score_half_points,
        available_half_points: color.as_white.available_half_points
            + color.as_black.available_half_points,
        wins,
        draws,
        losses,
        by_color: color,
        statistics: GameStatistics::from_observations(observations).into(),
        opening_results,
    }
}

fn summarize_candidate(label: String, versus: Vec<PairingResult>) -> CandidateResult {
    let mut color = ColorSplit::default();
    let mut observations = Vec::new();
    for pairing in &versus {
        add_color(&mut color.as_white, &pairing.by_color.as_white);
        add_color(&mut color.as_black, &pairing.by_color.as_black);
        observations.extend(pairing.opening_results.iter().flat_map(pair_observations));
    }
    let (wins, draws, losses) = totals(&color);
    CandidateResult {
        candidate_label: label,
        score_half_points: color.as_white.score_half_points + color.as_black.score_half_points,
        available_half_points: color.as_white.available_half_points
            + color.as_black.available_half_points,
        wins,
        draws,
        losses,
        by_color: color,
        statistics: GameStatistics::from_observations(observations).into(),
        versus,
    }
}

fn pair_observations(pair: &OpeningPairResult) -> impl Iterator<Item = GameObservation> {
    [
        observation(pair.candidate_as_white),
        observation(pair.candidate_as_black),
    ]
    .into_iter()
}

fn observation(value: SerializableObservation) -> GameObservation {
    use crate::self_play::DrawReason;
    let outcome = match (value.outcome, value.draw_reason) {
        ("white_win", _) => GameOutcome::WhiteWin,
        ("black_win", _) => GameOutcome::BlackWin,
        (_, Some("stalemate")) => GameOutcome::Draw(DrawReason::Stalemate),
        (_, Some("insufficient_material")) => GameOutcome::Draw(DrawReason::InsufficientMaterial),
        (_, Some("threefold_repetition")) => GameOutcome::Draw(DrawReason::ThreefoldRepetition),
        (_, Some("fifty_move_rule")) => GameOutcome::Draw(DrawReason::FiftyMoveRule),
        _ => GameOutcome::Draw(DrawReason::MaxPlies),
    };
    GameObservation {
        outcome,
        plies: value.plies,
    }
}

fn record_color(result: &mut ColorResult, game: SerializableObservation, candidate_white: bool) {
    result.games += 1;
    result.available_half_points += 2;
    match outcome_for_candidate(game, candidate_white) {
        2 => {
            result.wins += 1;
            result.score_half_points += 2;
        }
        1 => {
            result.draws += 1;
            result.score_half_points += 1;
        }
        _ => result.losses += 1,
    }
}

fn outcome_for_candidate(game: SerializableObservation, candidate_white: bool) -> u32 {
    match (game.outcome, candidate_white) {
        ("draw", _) => 1,
        ("white_win", true) | ("black_win", false) => 2,
        _ => 0,
    }
}

fn add_color(target: &mut ColorResult, value: &ColorResult) {
    target.games += value.games;
    target.score_half_points += value.score_half_points;
    target.available_half_points += value.available_half_points;
    target.wins += value.wins;
    target.draws += value.draws;
    target.losses += value.losses;
}

fn totals(color: &ColorSplit) -> (usize, usize, usize) {
    (
        color.as_white.wins + color.as_black.wins,
        color.as_white.draws + color.as_black.draws,
        color.as_white.losses + color.as_black.losses,
    )
}

fn score_as_white(outcome: GameOutcome) -> u32 {
    match outcome {
        GameOutcome::WhiteWin => 2,
        GameOutcome::BlackWin => 0,
        GameOutcome::Draw(_) => 1,
    }
}

fn score_as_black(outcome: GameOutcome) -> u32 {
    2 - score_as_white(outcome)
}

pub fn write_report(path: &Path, report: &RetentionReport) -> Result<(), RetentionError> {
    write_json_atomically(path, report).map_err(RetentionError::Persistence)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        encounter::GameRunnerFactory,
        evolution::{EvaluatedIndividual, EvolutionConfig, EvolutionState, GenerationResult},
        pairing::IndividualId,
        persistence::write_checkpoint,
        self_play::GameRecord,
        Genome,
    };
    use shakmaty::{Chess, Position};

    #[derive(Clone, Copy)]
    struct WhiteAlwaysWins;

    impl GameRunnerFactory for WhiteAlwaysWins {
        type Runner = Self;
        fn create(&self) -> Self::Runner {
            *self
        }
    }

    impl GameRunner for WhiteAlwaysWins {
        type Error = std::convert::Infallible;
        fn play(
            &mut self,
            _white: &Genome,
            _black: &Genome,
            _opening: &Opening,
            _search_depth: usize,
            _max_game_plies: usize,
        ) -> Result<GameRecord, Self::Error> {
            let position = Chess::default();
            Ok(GameRecord {
                outcome: GameOutcome::WhiteWin,
                moves: position.legal_moves().into_iter().take(1).collect(),
                position_history: vec![position.clone()],
                final_position: position,
            })
        }
    }

    fn individual(label: &str, id: u64, gene: usize) -> ResolvedIndividual {
        let mut genes = [0.1; GENE_COUNT];
        genes[gene] = 1.0;
        ResolvedIndividual {
            label: label.into(),
            checkpoint: PathBuf::from(format!("{label}.json")),
            selector: GenerationSelector::Generation {
                human_generation: 5,
            },
            resolved_generation: 5,
            training_seed: 123,
            checkpoint_sha256: format!("{id:064x}"),
            individual: Individual::new(IndividualId(id), Genome::new(genes).unwrap()),
        }
    }

    fn manifest(
        workers: usize,
        candidates: &[ResolvedIndividual],
        opponents: &[ResolvedIndividual],
    ) -> RetentionManifest {
        let selection = |value: &ResolvedIndividual| CheckpointSelection {
            label: value.label.clone(),
            checkpoint: value.checkpoint.clone(),
            generation: value.selector.clone(),
        };
        RetentionManifest {
            config: RetentionConfig {
                search_depth: 1,
                opening_pairs_per_opponent: 2,
                max_game_plies: 4,
                opening_min_plies: 2,
                opening_max_plies: 2,
                max_opening_attempts: 10,
                seed: 991,
                workers,
            },
            candidates: candidates.iter().map(selection).collect(),
            opponents: opponents.iter().map(selection).collect(),
        }
    }

    #[test]
    fn common_openings_color_reversal_and_half_points_are_reported() {
        let candidates = vec![individual("a", 1, 0), individual("b", 2, 1)];
        let opponents = vec![individual("old", 3, 2)];
        let report = run_retention(
            &manifest(2, &candidates, &opponents),
            &candidates,
            &opponents,
            WhiteAlwaysWins,
        )
        .unwrap();
        assert_eq!(report.total_games, 8);
        assert_eq!(report.opponent_openings.len(), 1);
        assert_eq!(report.opponent_openings[0].openings.len(), 2);
        for result in &report.candidate_results {
            assert_eq!((result.wins, result.draws, result.losses), (2, 0, 2));
            assert_eq!(
                (result.score_half_points, result.available_half_points),
                (4, 8)
            );
            assert_eq!(
                (
                    result.by_color.as_white.wins,
                    result.by_color.as_black.losses
                ),
                (2, 2)
            );
            assert_eq!(
                result.versus[0]
                    .opening_results
                    .iter()
                    .map(|opening| (opening.opening_id, opening.opening_seed))
                    .collect::<Vec<_>>(),
                report.opponent_openings[0]
                    .openings
                    .iter()
                    .map(|opening| (opening.opening_id, opening.opening_seed))
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn results_are_worker_and_candidate_order_independent() {
        let candidates = vec![individual("a", 1, 0), individual("b", 2, 1)];
        let opponents = vec![individual("old", 3, 2), individual("new", 4, 3)];
        let one = run_retention(
            &manifest(1, &candidates, &opponents),
            &candidates,
            &opponents,
            WhiteAlwaysWins,
        )
        .unwrap();
        let mut reversed = candidates.clone();
        reversed.reverse();
        let many = run_retention(
            &manifest(4, &reversed, &opponents),
            &reversed,
            &opponents,
            WhiteAlwaysWins,
        )
        .unwrap();
        let keyed = |report: &RetentionReport| {
            report
                .candidate_results
                .iter()
                .map(|result| {
                    (
                        result.candidate_label.clone(),
                        serde_json::to_value(result).unwrap(),
                    )
                })
                .collect::<BTreeMap<_, _>>()
        };
        assert_eq!(keyed(&one), keyed(&many));
        assert_eq!(one.opponent_openings, many.opponent_openings);

        assert_ne!(
            one.opponent_openings[0]
                .openings
                .iter()
                .map(|opening| opening.opening_seed)
                .collect::<Vec<_>>(),
            one.opponent_openings[1]
                .openings
                .iter()
                .map(|opening| opening.opening_seed)
                .collect::<Vec<_>>()
        );
        for result in &one.candidate_results {
            for pairing in &result.versus {
                let pool = one
                    .opponent_openings
                    .iter()
                    .find(|pool| pool.opponent_label == pairing.opponent_label)
                    .unwrap();
                assert_eq!(
                    pairing
                        .opening_results
                        .iter()
                        .map(|opening| opening.opening_seed)
                        .collect::<Vec<_>>(),
                    pool.openings
                        .iter()
                        .map(|opening| opening.opening_seed)
                        .collect::<Vec<_>>()
                );
            }
        }

        let mut reordered_opponents = opponents.clone();
        reordered_opponents.reverse();
        let reordered = run_retention(
            &manifest(3, &candidates, &reordered_opponents),
            &candidates,
            &reordered_opponents,
            WhiteAlwaysWins,
        )
        .unwrap();
        let opening_map = |report: &RetentionReport| {
            report
                .opponent_openings
                .iter()
                .map(|pool| {
                    (
                        pool.opponent_label.clone(),
                        serde_json::to_value(pool).unwrap(),
                    )
                })
                .collect::<BTreeMap<_, _>>()
        };
        assert_eq!(opening_map(&one), opening_map(&reordered));
        assert_eq!(one.total_games, 16);
    }

    #[test]
    fn report_has_versioned_round_trip_structure() {
        let candidates = vec![individual("a", 1, 0)];
        let opponents = vec![individual("old", 2, 1)];
        let report = run_retention(
            &manifest(1, &candidates, &opponents),
            &candidates,
            &opponents,
            WhiteAlwaysWins,
        )
        .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).unwrap();
        assert_eq!(json["format"], RETENTION_FORMAT);
        assert_eq!(json["version"], RETENTION_VERSION);
        assert_eq!(json["opening_derivation"]["candidate_independent"], true);
        assert_eq!(
            json["candidates"][0]["checkpoint_sha256"]
                .as_str()
                .unwrap()
                .len(),
            64
        );
        assert_eq!(json["opponent_openings"].as_array().unwrap().len(), 1);
        assert_eq!(json["total_opening_positions"], 2);
        assert_eq!(
            json["candidate_results"][0]["versus"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn rejects_empty_sets_duplicate_labels_and_zero_human_generation() {
        let item = CheckpointSelection {
            label: "same".into(),
            checkpoint: "x.json".into(),
            generation: GenerationSelector::Generation {
                human_generation: 0,
            },
        };
        let empty = RetentionManifest {
            config: manifest(1, &[individual("a", 1, 0)], &[individual("b", 2, 1)]).config,
            candidates: vec![],
            opponents: vec![item.clone()],
        };
        assert!(matches!(
            validate_manifest(&empty),
            Err(RetentionError::EmptyCandidates)
        ));
        let duplicate = RetentionManifest {
            candidates: vec![CheckpointSelection {
                generation: GenerationSelector::Generation {
                    human_generation: 1,
                },
                ..item.clone()
            }],
            opponents: vec![CheckpointSelection {
                generation: GenerationSelector::Generation {
                    human_generation: 1,
                },
                ..item
            }],
            ..empty
        };
        assert!(matches!(
            validate_manifest(&duplicate),
            Err(RetentionError::DuplicateLabel(_))
        ));
    }

    #[test]
    fn rejects_seed_collisions_and_duplicate_resolved_evaluators() {
        let a = individual("a", 1, 0);
        let b_same_phenotype = individual("b", 2, 0);
        let opponent = individual("opponent", 3, 1);
        assert!(matches!(
            validate_resolved(
                991,
                &[a.clone(), b_same_phenotype],
                std::slice::from_ref(&opponent)
            ),
            Err(RetentionError::DuplicateCandidatePhenotype { .. })
        ));
        let opponent_same_phenotype = individual("opponent-2", 4, 1);
        assert!(matches!(
            validate_resolved(
                991,
                std::slice::from_ref(&a),
                &[opponent.clone(), opponent_same_phenotype]
            ),
            Err(RetentionError::DuplicateOpponentPhenotype { .. })
        ));
        let duplicated = ResolvedIndividual {
            label: "same-individual".into(),
            ..a.clone()
        };
        assert!(matches!(
            validate_resolved(991, std::slice::from_ref(&a), &[duplicated]),
            Err(RetentionError::DuplicateResolvedIndividual { .. })
        ));
        assert!(matches!(
            validate_resolved(123, &[a], &[opponent]),
            Err(RetentionError::SeedCollision { .. })
        ));
        let candidate = individual("validation-candidate", 5, 2);
        let opponent = individual("validation-opponent", 6, 3);
        assert!(matches!(
            validate_resolved(
                ValidationConfig::default().master_seed(),
                &[candidate],
                &[opponent]
            ),
            Err(RetentionError::SeedCollision { .. })
        ));
    }

    #[test]
    fn resolves_one_based_generations_from_multiple_checkpoints() {
        fn checkpoint(path: &Path, id_base: u64) {
            let genome = |gene: usize| {
                let mut genes = [0.1; GENE_COUNT];
                genes[gene] = 1.0;
                Genome::new(genes).unwrap()
            };
            let generation = |index: usize| {
                GenerationResult::new(
                    index,
                    (0..32)
                        .map(|offset| {
                            EvaluatedIndividual::new(
                                Individual::new(
                                    IndividualId(id_base + index as u64 * 32 + offset),
                                    genome(index),
                                ),
                                crate::pairing::Score(64 - offset as u32),
                            )
                        })
                        .collect(),
                )
                .unwrap()
            };
            let generations = vec![generation(0), generation(1)];
            let best = generations[1].best().clone();
            let population = (0..32)
                .map(|offset| Individual::new(IndividualId(id_base + 64 + offset), genome(2)))
                .collect();
            let state =
                EvolutionState::new(2, population, generations, best, id_base + 96, 7).unwrap();
            write_checkpoint(path, &EvolutionConfig::default(), &state).unwrap();
        }

        let directory = std::env::temp_dir();
        let left = directory.join(format!("blocky-retention-{}-left.json", std::process::id()));
        let right = directory.join(format!(
            "blocky-retention-{}-right.json",
            std::process::id()
        ));
        checkpoint(&left, 0);
        checkpoint(&right, 1000);
        let resolved = resolve_selections(&[
            CheckpointSelection {
                label: "left-g1".into(),
                checkpoint: left.clone(),
                generation: GenerationSelector::Generation {
                    human_generation: 1,
                },
            },
            CheckpointSelection {
                label: "right-g2".into(),
                checkpoint: right.clone(),
                generation: GenerationSelector::Generation {
                    human_generation: 2,
                },
            },
        ])
        .unwrap();
        assert_eq!(resolved[0].resolved_generation, 1);
        assert_eq!(resolved[0].individual.id().0, 0);
        assert_eq!(resolved[1].resolved_generation, 2);
        assert_eq!(resolved[1].individual.id().0, 1032);
        assert_eq!(
            resolved[0].checkpoint_sha256,
            hex_digest(&fs::read(&left).unwrap())
        );
        assert_eq!(resolved[0].checkpoint_sha256.len(), 64);
        fs::remove_file(left).unwrap();
        fs::remove_file(right).unwrap();
    }
}
