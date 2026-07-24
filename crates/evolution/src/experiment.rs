//! Application service that composes evolution and external validation.

use std::{error::Error, fmt};

use crate::{
    encounter::ProductionGameRunner,
    evolution::{
        EvolutionConfig, EvolutionEngine, EvolutionError, EvolutionResult, PopulationEvaluator,
        SelfPlayPopulationEvaluator,
    },
    genome::Genome,
    validation::{ChampionValidator, ValidationConfig, ValidationError, ValidationReport},
};

/// Boundary used by the experiment service to obtain an evolutionary result.
pub trait EvolutionRunner {
    type Error;

    fn evolve(&mut self) -> Result<EvolutionResult, Self::Error>;
}

impl<E: PopulationEvaluator> EvolutionRunner for EvolutionEngine<E> {
    type Error = EvolutionError<E::Error>;

    fn evolve(&mut self) -> Result<EvolutionResult, Self::Error> {
        self.run()
    }
}

/// Boundary used to keep candidate validation replaceable and fast in tests.
pub trait CandidateValidator {
    type Error;

    fn validate_candidate(&mut self, candidate: &Genome) -> Result<ValidationReport, Self::Error>;
}

impl<R: crate::encounter::ConfiguredGameRunner> CandidateValidator for ChampionValidator<R> {
    type Error = ValidationError<R::Error>;

    fn validate_candidate(&mut self, candidate: &Genome) -> Result<ValidationReport, Self::Error> {
        self.validate(candidate)
    }
}

/// Complete in-memory outcome. The candidate is not duplicated: it remains
/// available through `evolution.best_ever()`.
#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentReport {
    evolution: EvolutionResult,
    validation: ValidationReport,
}

impl ExperimentReport {
    pub const fn evolution(&self) -> &EvolutionResult {
        &self.evolution
    }
    pub const fn candidate(&self) -> &Genome {
        self.evolution.best_ever().individual().genome()
    }
    pub const fn validation(&self) -> &ValidationReport {
        &self.validation
    }
    pub const fn accepted(&self) -> bool {
        self.validation.accepted
    }
}

pub struct ExperimentService<T, V> {
    trainer: T,
    validator: V,
}

impl<T, V> ExperimentService<T, V> {
    pub fn new(trainer: T, validator: V) -> Self {
        Self { trainer, validator }
    }
}

impl<T: EvolutionRunner, V: CandidateValidator> ExperimentService<T, V> {
    pub fn run(&mut self) -> Result<ExperimentReport, ExperimentError<T::Error, V::Error>> {
        let evolution = self.trainer.evolve().map_err(ExperimentError::Evolution)?;
        let validation = self
            .validator
            .validate_candidate(evolution.best_ever().individual().genome())
            .map_err(ExperimentError::Validation)?;
        Ok(ExperimentReport {
            evolution,
            validation,
        })
    }
}

pub type ProductionExperimentService = ExperimentService<
    EvolutionEngine<SelfPlayPopulationEvaluator<ProductionGameRunner>>,
    ChampionValidator<ProductionGameRunner>,
>;

impl ProductionExperimentService {
    pub fn production(
        evolution: EvolutionConfig,
        validation: ValidationConfig,
    ) -> Result<Self, ExperimentConfigError> {
        if evolution.training().master_seed() == validation.master_seed() {
            return Err(ExperimentConfigError::SeedCollision(
                validation.master_seed(),
            ));
        }
        let trainer = EvolutionEngine::with_defaults(
            evolution,
            SelfPlayPopulationEvaluator::new(ProductionGameRunner),
        );
        Ok(Self::new(
            trainer,
            ChampionValidator::production(validation),
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExperimentConfigError {
    SeedCollision(u64),
}

impl fmt::Display for ExperimentConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SeedCollision(seed) => write!(
                formatter,
                "training and held-out validation must use separate seeds, both were {seed}"
            ),
        }
    }
}
impl Error for ExperimentConfigError {}

#[derive(Debug, PartialEq)]
pub enum ExperimentError<T, V> {
    Evolution(T),
    Validation(V),
}

impl<T: fmt::Display, V: fmt::Display> fmt::Display for ExperimentError<T, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(source) => write!(formatter, "evolution failed: {source}"),
            Self::Validation(source) => write!(formatter, "validation failed: {source}"),
        }
    }
}

impl<T: Error + 'static, V: Error + 'static> Error for ExperimentError<T, V> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Evolution(source) => Some(source),
            Self::Validation(source) => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, convert::Infallible, rc::Rc};

    use super::*;
    use crate::{
        evolution::Individual,
        pairing::{IndividualId, Score, Standing},
        training::TrainingConfig,
    };

    struct RankedEvaluator;

    impl PopulationEvaluator for RankedEvaluator {
        type Error = Infallible;

        fn evaluate(
            &mut self,
            _generation: usize,
            population: &[Individual],
            _config: &EvolutionConfig,
        ) -> Result<Vec<Standing>, Self::Error> {
            Ok(population
                .iter()
                .map(|individual| Standing {
                    individual: individual.id(),
                    score: Score(individual.id().0 as u32),
                })
                .collect())
        }
    }

    struct SpyValidator {
        seen: Rc<RefCell<Vec<Genome>>>,
        result: Result<ValidationReport, &'static str>,
    }

    impl CandidateValidator for SpyValidator {
        type Error = &'static str;

        fn validate_candidate(
            &mut self,
            candidate: &Genome,
        ) -> Result<ValidationReport, Self::Error> {
            self.seen.borrow_mut().push(candidate.clone());
            self.result.clone()
        }
    }

    fn evolution_config(training_seed: u64) -> EvolutionConfig {
        EvolutionConfig::new(
            TrainingConfig::new(1, 1, training_seed, 0..=0, 1).unwrap(),
            1,
            4,
            1,
            1,
            2,
            0.15,
            0.02,
            0.1,
            0.5,
        )
        .unwrap()
    }

    fn validation_report(accepted: bool) -> ValidationReport {
        ValidationReport {
            config: ValidationConfig::new(vec![1], 1, 1, 99, 0..=0, 1, 1).unwrap(),
            by_depth: vec![],
            candidate_score: Score(5),
            reference_score: Score(3),
            accepted,
        }
    }

    #[test]
    fn validates_exactly_best_ever_and_preserves_its_training_fitness() {
        let seen = Rc::new(RefCell::new(vec![]));
        let trainer = EvolutionEngine::with_defaults(evolution_config(1), RankedEvaluator);
        let validator = SpyValidator {
            seen: Rc::clone(&seen),
            result: Ok(validation_report(true)),
        };
        let mut service = ExperimentService::new(trainer, validator);

        let report = service.run().unwrap();

        assert_eq!(seen.borrow().as_slice(), &[report.candidate().clone()]);
        assert_eq!(report.evolution().best_ever().fitness(), Score(3));
        assert_eq!(
            report.evolution().best_ever().individual().id(),
            IndividualId(3)
        );
        assert!(report.accepted());
    }

    struct FailingTrainer;

    impl EvolutionRunner for FailingTrainer {
        type Error = &'static str;

        fn evolve(&mut self) -> Result<EvolutionResult, Self::Error> {
            Err("training")
        }
    }

    #[test]
    fn propagates_errors_and_does_not_validate_after_training_failure() {
        let seen = Rc::new(RefCell::new(vec![]));
        let validator = SpyValidator {
            seen: Rc::clone(&seen),
            result: Ok(validation_report(false)),
        };
        let mut service = ExperimentService::new(FailingTrainer, validator);

        assert_eq!(service.run(), Err(ExperimentError::Evolution("training")));
        assert!(seen.borrow().is_empty());

        let trainer = EvolutionEngine::with_defaults(evolution_config(1), RankedEvaluator);
        let validator = SpyValidator {
            seen,
            result: Err("validation"),
        };
        let mut service = ExperimentService::new(trainer, validator);
        assert!(matches!(
            service.run(),
            Err(ExperimentError::Validation("validation"))
        ));
    }

    #[test]
    fn production_service_requires_independent_training_and_validation_seeds() {
        let validation = ValidationConfig::new(vec![1], 1, 1, 7, 0..=0, 1, 1).unwrap();
        assert!(matches!(
            ProductionExperimentService::production(evolution_config(7), validation),
            Err(ExperimentConfigError::SeedCollision(7))
        ));
    }
}
