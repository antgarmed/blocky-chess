//! Thin command-line adapter for configuring and reporting an experiment.

use std::{error::Error, fmt, ops::RangeInclusive};

use crate::{
    evolution::{EvolutionConfig, EvolutionConfigError},
    experiment::ExperimentReport,
    progress::{ProgressEvent, ProgressObserver},
    training::{TrainingConfig, TrainingConfigError},
    validation::{ValidationConfig, ValidationConfigError},
};

pub const HELP: &str = "\
Train Blocky Chess evaluation parameters through deterministic self-play

Usage:
  blocky-evolution train [OPTIONS]
  blocky-evolution --help

Evolution:
  --generations N                         [default: 100]
  --population-size N                     [default: 32]
  --swiss-rounds N                        [default: 5]
  --elite-count N                         [default: 2]
  --parent-candidate-count N              [default: 3]
  --gene-mutation-probability P           [default: 0.15]
  --strong-mutation-probability P         [default: 0.02]
  --mutation-step P                       [default: 0.10]
  --strong-mutation-step P                [default: 0.50]

Training games:
  --search-depth N                        [default: 4]
  --max-game-plies N                      [default: 200]
  --training-seed N                       [default: 0]
  --opening-min-plies N                   [default: 4]
  --opening-max-plies N                   [default: 10]
  --max-opening-attempts N                [default: 100]

Champion validation:
  --validation-depths N,N                 [default: 4,6]
  --validation-openings N                 [default: 20]
  --validation-max-game-plies N           [default: 200]
  --validation-seed N                     [default: 6215332838309450821]
  --validation-opening-min-plies N        [default: 4]
  --validation-opening-max-plies N        [default: 10]
  --validation-max-opening-attempts N     [default: 100]
  --validation-minimum-margin-half-points N [default: 1]

  -h, --help                              Print help
";

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Help,
    Train(Box<TrainCommand>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrainCommand {
    pub evolution: EvolutionConfig,
    pub validation: ValidationConfig,
}

impl TrainCommand {
    pub fn from_args<I, S>(args: I) -> Result<Command, CliError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        match args.first().map(String::as_str) {
            Some("-h" | "--help") => return Ok(Command::Help),
            Some("train") => {}
            Some(command) => return Err(CliError::UnknownCommand(command.to_owned())),
            None => return Err(CliError::MissingCommand),
        }

        let mut values = RawValues::default();
        let mut index = 1;
        while index < args.len() {
            let flag = &args[index];
            if flag == "-h" || flag == "--help" {
                return Ok(Command::Help);
            }
            let value = args
                .get(index + 1)
                .ok_or_else(|| CliError::MissingValue(flag.clone()))?;
            values.set(flag, value)?;
            index += 2;
        }
        values.build().map(Box::new).map(Command::Train)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CliError {
    MissingCommand,
    UnknownCommand(String),
    UnknownOption(String),
    MissingValue(String),
    InvalidValue {
        option: String,
        value: String,
        expected: &'static str,
    },
    TrainingConfig(TrainingConfigError),
    EvolutionConfig(EvolutionConfigError),
    ValidationConfig(ValidationConfigError),
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => formatter.write_str("missing command; use `train` or `--help`"),
            Self::UnknownCommand(command) => {
                write!(formatter, "unknown command `{command}`; use `--help`")
            }
            Self::UnknownOption(option) => {
                write!(formatter, "unknown option `{option}`; use `train --help`")
            }
            Self::MissingValue(option) => write!(formatter, "option `{option}` requires a value"),
            Self::InvalidValue {
                option,
                value,
                expected,
            } => write!(
                formatter,
                "invalid value `{value}` for `{option}`; expected {expected}"
            ),
            Self::TrainingConfig(source) => {
                write!(formatter, "invalid training configuration: {source}")
            }
            Self::EvolutionConfig(source) => {
                write!(
                    formatter,
                    "invalid evolution configuration: {}",
                    evolution_error(source)
                )
            }
            Self::ValidationConfig(source) => {
                write!(
                    formatter,
                    "invalid validation configuration: {}",
                    validation_error(source)
                )
            }
        }
    }
}

impl Error for CliError {}

fn evolution_error(error: &EvolutionConfigError) -> String {
    match error {
        EvolutionConfigError::ZeroGenerations => "generations must be greater than zero".into(),
        EvolutionConfigError::PopulationTooSmall(size) => {
            format!("population size must be at least 2, got {size}")
        }
        EvolutionConfigError::OddPopulation(size) => {
            format!("population size must be even, got {size}")
        }
        EvolutionConfigError::ZeroSwissRounds => {
            "Swiss rounds must be greater than zero".into()
        }
        EvolutionConfigError::TooManySwissRounds { rounds, population } => format!(
            "Swiss rounds must be smaller than population size, got {rounds} rounds and {population} individuals"
        ),
        EvolutionConfigError::TooManyElites {
            elites,
            population,
        } => format!(
            "elite count must be smaller than population size, got {elites} elites and {population} individuals"
        ),
        EvolutionConfigError::InvalidParentCandidateCount {
            candidates,
            population,
        } => format!(
            "parent candidate count must be between 1 and {}, got {candidates}",
            population.saturating_sub(1)
        ),
        EvolutionConfigError::InvalidProbability { name, value } => {
            format!("{name} probability must be between 0 and 1, got {value}")
        }
        EvolutionConfigError::InvalidMutationStep { name, value } => {
            format!("{name} step must be finite and greater than zero, got {value}")
        }
    }
}

fn validation_error(error: &ValidationConfigError) -> String {
    match error {
        ValidationConfigError::NoSearchDepths => {
            "validation depths must contain at least one depth".into()
        }
        ValidationConfigError::ZeroSearchDepth => {
            "validation depths must be greater than zero".into()
        }
        ValidationConfigError::DuplicateSearchDepth => {
            "validation depths must not contain duplicates".into()
        }
        ValidationConfigError::ZeroOpenings => {
            "validation openings must be greater than zero".into()
        }
        ValidationConfigError::Training(source) => source.to_string(),
    }
}

#[derive(Clone, Debug)]
struct RawValues {
    generations: usize,
    population_size: usize,
    swiss_rounds: usize,
    elite_count: usize,
    parent_candidate_count: usize,
    gene_mutation_probability: f64,
    strong_mutation_probability: f64,
    mutation_step: f64,
    strong_mutation_step: f64,
    search_depth: usize,
    max_game_plies: usize,
    training_seed: u64,
    opening_min_plies: usize,
    opening_max_plies: usize,
    max_opening_attempts: usize,
    validation_depths: Vec<usize>,
    validation_openings: usize,
    validation_max_game_plies: usize,
    validation_seed: u64,
    validation_opening_min_plies: usize,
    validation_opening_max_plies: usize,
    validation_max_opening_attempts: usize,
    validation_minimum_margin_half_points: u32,
}

impl Default for RawValues {
    fn default() -> Self {
        let evolution = EvolutionConfig::default();
        let training = evolution.training();
        let validation = ValidationConfig::default();
        Self {
            generations: evolution.generations(),
            population_size: evolution.population_size(),
            swiss_rounds: evolution.swiss_rounds(),
            elite_count: evolution.elite_count(),
            parent_candidate_count: evolution.parent_candidate_count(),
            gene_mutation_probability: evolution.gene_mutation_probability(),
            strong_mutation_probability: evolution.strong_mutation_probability(),
            mutation_step: evolution.mutation_step(),
            strong_mutation_step: evolution.strong_mutation_step(),
            search_depth: training.search_depth(),
            max_game_plies: training.max_game_plies(),
            training_seed: training.master_seed(),
            opening_min_plies: *training.opening_plies().start(),
            opening_max_plies: *training.opening_plies().end(),
            max_opening_attempts: training.max_opening_attempts(),
            validation_depths: validation.search_depths().to_vec(),
            validation_openings: validation.opening_count(),
            validation_max_game_plies: validation.max_game_plies(),
            validation_seed: validation.master_seed(),
            validation_opening_min_plies: *validation.opening_plies().start(),
            validation_opening_max_plies: *validation.opening_plies().end(),
            validation_max_opening_attempts: validation.max_opening_attempts(),
            validation_minimum_margin_half_points: validation.minimum_margin_half_points(),
        }
    }
}

impl RawValues {
    fn set(&mut self, option: &str, value: &str) -> Result<(), CliError> {
        macro_rules! number {
            ($field:ident, $expected:literal) => {
                self.$field = parse(option, value, $expected)?
            };
        }
        match option {
            "--generations" => number!(generations, "a non-negative integer"),
            "--population-size" => number!(population_size, "a non-negative integer"),
            "--swiss-rounds" => number!(swiss_rounds, "a non-negative integer"),
            "--elite-count" => number!(elite_count, "a non-negative integer"),
            "--parent-candidate-count" => {
                number!(parent_candidate_count, "a non-negative integer")
            }
            "--gene-mutation-probability" => number!(gene_mutation_probability, "a number"),
            "--strong-mutation-probability" => {
                number!(strong_mutation_probability, "a number")
            }
            "--mutation-step" => number!(mutation_step, "a number"),
            "--strong-mutation-step" => number!(strong_mutation_step, "a number"),
            "--search-depth" => number!(search_depth, "a non-negative integer"),
            "--max-game-plies" => number!(max_game_plies, "a non-negative integer"),
            "--training-seed" => number!(training_seed, "an unsigned 64-bit integer"),
            "--opening-min-plies" => number!(opening_min_plies, "a non-negative integer"),
            "--opening-max-plies" => number!(opening_max_plies, "a non-negative integer"),
            "--max-opening-attempts" => {
                number!(max_opening_attempts, "a non-negative integer")
            }
            "--validation-depths" => {
                self.validation_depths = parse_depths(option, value)?;
            }
            "--validation-openings" => {
                number!(validation_openings, "a non-negative integer")
            }
            "--validation-max-game-plies" => {
                number!(validation_max_game_plies, "a non-negative integer")
            }
            "--validation-seed" => number!(validation_seed, "an unsigned 64-bit integer"),
            "--validation-opening-min-plies" => {
                number!(validation_opening_min_plies, "a non-negative integer")
            }
            "--validation-opening-max-plies" => {
                number!(validation_opening_max_plies, "a non-negative integer")
            }
            "--validation-max-opening-attempts" => {
                number!(validation_max_opening_attempts, "a non-negative integer")
            }
            "--validation-minimum-margin-half-points" => {
                number!(
                    validation_minimum_margin_half_points,
                    "an unsigned 32-bit integer"
                )
            }
            _ => return Err(CliError::UnknownOption(option.to_owned())),
        }
        Ok(())
    }

    fn build(self) -> Result<TrainCommand, CliError> {
        let training = TrainingConfig::new(
            self.search_depth,
            self.max_game_plies,
            self.training_seed,
            range(self.opening_min_plies, self.opening_max_plies),
            self.max_opening_attempts,
        )
        .map_err(CliError::TrainingConfig)?;
        let evolution = EvolutionConfig::new(
            training,
            self.generations,
            self.population_size,
            self.swiss_rounds,
            self.elite_count,
            self.parent_candidate_count,
            self.gene_mutation_probability,
            self.strong_mutation_probability,
            self.mutation_step,
            self.strong_mutation_step,
        )
        .map_err(CliError::EvolutionConfig)?;
        let validation = ValidationConfig::new(
            self.validation_depths,
            self.validation_openings,
            self.validation_max_game_plies,
            self.validation_seed,
            range(
                self.validation_opening_min_plies,
                self.validation_opening_max_plies,
            ),
            self.validation_max_opening_attempts,
            self.validation_minimum_margin_half_points,
        )
        .map_err(CliError::ValidationConfig)?;
        Ok(TrainCommand {
            evolution,
            validation,
        })
    }
}

fn range(start: usize, end: usize) -> RangeInclusive<usize> {
    start..=end
}

fn parse<T: std::str::FromStr>(
    option: &str,
    value: &str,
    expected: &'static str,
) -> Result<T, CliError> {
    value.parse().map_err(|_| CliError::InvalidValue {
        option: option.to_owned(),
        value: value.to_owned(),
        expected,
    })
}

fn parse_depths(option: &str, value: &str) -> Result<Vec<usize>, CliError> {
    if value.is_empty() {
        return Err(CliError::InvalidValue {
            option: option.to_owned(),
            value: value.to_owned(),
            expected: "a comma-separated list of positive integers",
        });
    }
    value
        .split(',')
        .map(|depth| parse(option, depth, "a comma-separated list of positive integers"))
        .collect()
}

pub fn render_summary(report: &ExperimentReport) -> String {
    let verdict = if report.accepted() {
        "accepted"
    } else {
        "rejected"
    };
    let validation = report.validation();
    let mut output = format!(
        "Experiment complete: champion {verdict}\n\
         Generations: {}\n\
         Validation score (half-points): candidate {}, reference {}\n",
        report.evolution().generations().len(),
        validation.candidate_score.0,
        validation.reference_score.0,
    );
    for depth in &validation.by_depth {
        let depth_verdict = if depth.accepted {
            "accepted"
        } else {
            "rejected"
        };
        output.push_str(&format!(
            "  Depth {}: candidate {}, reference {} ({})\n",
            depth.search_depth, depth.candidate_score.0, depth.reference_score.0, depth_verdict
        ));
    }
    output
}

/// Human-readable progress adapter for interactive command-line runs.
#[derive(Default)]
pub struct ConsoleProgressObserver;

impl ProgressObserver for ConsoleProgressObserver {
    fn on_event(&mut self, event: ProgressEvent) {
        eprintln!("{}", render_progress(event));
    }
}

pub fn render_progress(event: ProgressEvent) -> String {
    match event {
        ProgressEvent::EvolutionStarted {
            generations,
            population_size,
        } => format!("Evolution started: {generations} generations, population {population_size}"),
        ProgressEvent::GenerationStarted {
            generation,
            total_generations,
        } => format!("Generation {}/{total_generations} started", generation + 1),
        ProgressEvent::SelfPlayRoundCompleted {
            generation,
            round,
            total_rounds,
            opening,
        } => format!(
            "Generation {}: Swiss round {}/{total_rounds} completed (opening {})",
            generation + 1,
            round + 1,
            opening.0
        ),
        ProgressEvent::GenerationCompleted {
            generation,
            total_generations,
            best,
            best_score,
        } => format!(
            "Generation {}/{total_generations} completed: best individual {}, score {}",
            generation + 1,
            best.0,
            best_score.points()
        ),
        ProgressEvent::EvolutionCompleted {
            generations,
            best,
            best_score,
        } => format!(
            "Evolution completed after {generations} generations: best individual {}, score {}",
            best.0,
            best_score.points()
        ),
        ProgressEvent::ValidationStarted {
            depth_count,
            openings_per_depth,
        } => format!(
            "Validation started: {depth_count} depths, {openings_per_depth} openings per depth"
        ),
        ProgressEvent::ValidationDepthStarted {
            search_depth,
            depth_index,
            total_depths,
        } => format!(
            "Validation depth {}/{total_depths} started (search depth {search_depth})",
            depth_index + 1
        ),
        ProgressEvent::ValidationOpeningCompleted {
            search_depth,
            opening_index,
            total_openings,
            opening,
        } => format!(
            "Validation depth {search_depth}: opening {}/{total_openings} completed (opening {})",
            opening_index + 1,
            opening.0
        ),
        ProgressEvent::ValidationDepthCompleted {
            search_depth,
            candidate_score,
            reference_score,
            accepted,
        } => format!(
            "Validation depth {search_depth} completed: candidate {}, reference {}, {}",
            candidate_score.points(),
            reference_score.points(),
            verdict(accepted)
        ),
        ProgressEvent::ValidationCompleted {
            candidate_score,
            reference_score,
            accepted,
        } => format!(
            "Validation completed: candidate {}, reference {}, {}",
            candidate_score.points(),
            reference_score.points(),
            verdict(accepted)
        ),
    }
}

fn verdict(accepted: bool) -> &'static str {
    if accepted {
        "accepted"
    } else {
        "rejected"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        openings::OpeningId,
        pairing::{IndividualId, Score},
    };

    fn train(args: &[&str]) -> TrainCommand {
        match TrainCommand::from_args(args.iter().copied()).unwrap() {
            Command::Train(command) => *command,
            Command::Help => panic!("expected train command"),
        }
    }

    #[test]
    fn no_options_use_domain_defaults() {
        let command = train(&["train"]);
        assert_eq!(command.evolution, EvolutionConfig::default());
        assert_eq!(command.validation, ValidationConfig::default());
    }

    #[test]
    fn every_hyperparameter_can_be_overridden() {
        let command = train(&[
            "train",
            "--generations",
            "2",
            "--population-size",
            "6",
            "--swiss-rounds",
            "2",
            "--elite-count",
            "1",
            "--parent-candidate-count",
            "2",
            "--gene-mutation-probability",
            "0.25",
            "--strong-mutation-probability",
            "0.05",
            "--mutation-step",
            "0.2",
            "--strong-mutation-step",
            "0.8",
            "--search-depth",
            "3",
            "--max-game-plies",
            "80",
            "--training-seed",
            "42",
            "--opening-min-plies",
            "2",
            "--opening-max-plies",
            "8",
            "--max-opening-attempts",
            "50",
            "--validation-depths",
            "3,5,7",
            "--validation-openings",
            "8",
            "--validation-max-game-plies",
            "100",
            "--validation-seed",
            "99",
            "--validation-opening-min-plies",
            "6",
            "--validation-opening-max-plies",
            "12",
            "--validation-max-opening-attempts",
            "60",
            "--validation-minimum-margin-half-points",
            "2",
        ]);
        let evolution = &command.evolution;
        let training = evolution.training();
        assert_eq!(evolution.generations(), 2);
        assert_eq!(evolution.population_size(), 6);
        assert_eq!(evolution.swiss_rounds(), 2);
        assert_eq!(evolution.elite_count(), 1);
        assert_eq!(evolution.parent_candidate_count(), 2);
        assert_eq!(evolution.gene_mutation_probability(), 0.25);
        assert_eq!(evolution.strong_mutation_probability(), 0.05);
        assert_eq!(evolution.mutation_step(), 0.2);
        assert_eq!(evolution.strong_mutation_step(), 0.8);
        assert_eq!(training.search_depth(), 3);
        assert_eq!(training.max_game_plies(), 80);
        assert_eq!(training.master_seed(), 42);
        assert_eq!(training.opening_plies(), &(2..=8));
        assert_eq!(training.max_opening_attempts(), 50);
        let validation = &command.validation;
        assert_eq!(validation.search_depths(), &[3, 5, 7]);
        assert_eq!(validation.opening_count(), 8);
        assert_eq!(validation.max_game_plies(), 100);
        assert_eq!(validation.master_seed(), 99);
        assert_eq!(validation.opening_plies(), &(6..=12));
        assert_eq!(validation.max_opening_attempts(), 60);
        assert_eq!(validation.minimum_margin_half_points(), 2);
    }

    #[test]
    fn help_works_at_both_command_levels() {
        assert_eq!(TrainCommand::from_args(["--help"]), Ok(Command::Help));
        assert_eq!(
            TrainCommand::from_args(["train", "--help"]),
            Ok(Command::Help)
        );
    }

    #[test]
    fn syntax_errors_are_specific() {
        assert_eq!(
            TrainCommand::from_args(Vec::<String>::new()),
            Err(CliError::MissingCommand)
        );
        assert_eq!(
            TrainCommand::from_args(["race"]),
            Err(CliError::UnknownCommand("race".into()))
        );
        assert_eq!(
            TrainCommand::from_args(["train", "--wat", "1"]),
            Err(CliError::UnknownOption("--wat".into()))
        );
        assert_eq!(
            TrainCommand::from_args(["train", "--generations"]),
            Err(CliError::MissingValue("--generations".into()))
        );
        assert!(matches!(
            TrainCommand::from_args(["train", "--generations", "many"]),
            Err(CliError::InvalidValue { .. })
        ));
    }

    #[test]
    fn invalid_combinations_are_rejected_by_domain_types() {
        assert!(matches!(
            TrainCommand::from_args(["train", "--population-size", "3"]),
            Err(CliError::EvolutionConfig(
                EvolutionConfigError::OddPopulation(3)
            ))
        ));
        assert!(matches!(
            TrainCommand::from_args(["train", "--validation-depths", "4,4"]),
            Err(CliError::ValidationConfig(
                ValidationConfigError::DuplicateSearchDepth
            ))
        ));
        assert!(matches!(
            TrainCommand::from_args([
                "train",
                "--opening-min-plies",
                "9",
                "--opening-max-plies",
                "7"
            ]),
            Err(CliError::TrainingConfig(
                TrainingConfigError::EmptyOpeningRange
            ))
        ));
    }

    #[test]
    fn renders_generation_and_round_progress_for_humans() {
        assert_eq!(
            render_progress(ProgressEvent::SelfPlayRoundCompleted {
                generation: 1,
                round: 2,
                total_rounds: 5,
                opening: OpeningId(7),
            }),
            "Generation 2: Swiss round 3/5 completed (opening 7)"
        );
        assert_eq!(
            render_progress(ProgressEvent::GenerationCompleted {
                generation: 1,
                total_generations: 10,
                best: IndividualId(11),
                best_score: Score(7),
            }),
            "Generation 2/10 completed: best individual 11, score 3.5"
        );
    }

    #[test]
    fn renders_validation_progress_and_verdict() {
        assert_eq!(
            render_progress(ProgressEvent::ValidationOpeningCompleted {
                search_depth: 6,
                opening_index: 3,
                total_openings: 20,
                opening: OpeningId(3),
            }),
            "Validation depth 6: opening 4/20 completed (opening 3)"
        );
        assert_eq!(
            render_progress(ProgressEvent::ValidationCompleted {
                candidate_score: Score(42),
                reference_score: Score(38),
                accepted: true,
            }),
            "Validation completed: candidate 21, reference 19, accepted"
        );
    }
}
