//! Genetic-training primitives for Blocky Chess.
//!
//! This crate deliberately keeps the evolutionary loop separate from the
//! reusable representation of an evaluation genome.

pub mod cli;
pub mod encounter;
pub mod evolution;
pub mod experiment;
pub mod genome;
pub mod openings;
pub mod pairing;
pub mod progress;
pub mod rng;
pub mod self_play;
pub mod training;
pub mod validation;

pub use genome::{
    Gene, Genome, GenomeError, EFFECTIVE_MOBILITY_WEIGHT, EVALUATION_QUANTIZATION_SCALE, GENE_COUNT,
};
