//! Progress reporting boundary for long-running experiments.

use crate::{
    openings::OpeningId,
    pairing::{IndividualId, Score},
    telemetry::GameStatistics,
};

/// A stable, domain-level description of work completed by an experiment.
///
/// Events deliberately contain only values that are already produced by the
/// algorithm. Observers therefore cannot participate in random decisions or
/// influence fitness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressEvent {
    EvolutionStarted {
        generations: usize,
        population_size: usize,
    },
    GenerationStarted {
        generation: usize,
        total_generations: usize,
    },
    SelfPlayRoundCompleted {
        generation: usize,
        round: usize,
        total_rounds: usize,
        opening: OpeningId,
        statistics: GameStatistics,
    },
    SelfPlayGenerationCompleted {
        generation: usize,
        statistics: GameStatistics,
    },
    DefaultAnchorCompleted {
        generation: usize,
        opening_pairs: usize,
        games: usize,
        candidate_half_points: u32,
        available_half_points: u32,
        maximum_selection_units: u32,
        statistics: GameStatistics,
    },
    GenerationCompleted {
        generation: usize,
        total_generations: usize,
        best: IndividualId,
        best_score: Score,
    },
    EvolutionCompleted {
        generations: usize,
        best: IndividualId,
        best_score: Score,
    },
    ValidationStarted {
        depth_count: usize,
        openings_per_depth: usize,
    },
    ValidationDepthStarted {
        search_depth: usize,
        depth_index: usize,
        total_depths: usize,
    },
    ValidationOpeningCompleted {
        search_depth: usize,
        opening_index: usize,
        total_openings: usize,
        opening: OpeningId,
    },
    ValidationDepthCompleted {
        search_depth: usize,
        candidate_score: Score,
        reference_score: Score,
        accepted: bool,
        statistics: GameStatistics,
    },
    ValidationCompleted {
        candidate_score: Score,
        reference_score: Score,
        accepted: bool,
    },
}

/// Receives progress notifications without owning presentation concerns.
pub trait ProgressObserver {
    fn on_event(&mut self, event: ProgressEvent);
}

/// Default observer for library callers that do not need progress reporting.
#[derive(Default)]
pub struct NoopProgressObserver;

impl ProgressObserver for NoopProgressObserver {
    fn on_event(&mut self, _event: ProgressEvent) {}
}
