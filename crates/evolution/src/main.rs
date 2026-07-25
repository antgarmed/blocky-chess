use std::{env, process::ExitCode};

use blocky_evolution::{
    cli::{
        render_summary, write_stdout_line, BenchmarkCommand, Command, ConsoleProgressObserver,
        TrainCommand, ValidateCommand, HELP,
    },
    encounter::ProductionGameRunner,
    evolution::{EvolutionEngine, SelfPlayPopulationEvaluator},
    experiment::ExperimentReport,
    persistence::{
        read_checkpoint, read_checkpoint_unchecked_config, write_benchmark_report,
        write_checkpoint, write_experiment_report, write_validation_report,
    },
    validation::{CandidateSelector, ChampionValidator},
};

fn main() -> ExitCode {
    let command = match TrainCommand::from_args(env::args().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("error: {error}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };
    match command {
        Command::Help => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Command::Train(command) => run_train(*command),
        Command::Validate(command) => run_validate(*command),
        Command::Benchmark(command) => run_benchmark(*command),
    }
}

fn run_benchmark(command: BenchmarkCommand) -> ExitCode {
    let (evolution_config, state) = match read_checkpoint_unchecked_config(&command.checkpoint) {
        Ok(data) => data,
        Err(error) => {
            eprintln!("error: could not read checkpoint: {error}");
            return ExitCode::from(2);
        }
    };
    let training_seed = evolution_config.training().master_seed();
    let final_validation_seed =
        blocky_evolution::validation::ValidationConfig::default().master_seed();
    let seeds = [
        training_seed,
        final_validation_seed,
        command.config.benchmark_seed,
        command.config.opponent_seed,
    ];
    for left in 0..seeds.len() {
        for right in left + 1..seeds.len() {
            if seeds[left] == seeds[right] {
                eprintln!("error: training, final validation, benchmark, and opponent seeds must be distinct");
                return ExitCode::from(2);
            }
        }
    }
    let candidate = match &command.selector {
        CandidateSelector::BestEver => state.best_ever(),
        CandidateSelector::Generation(human) => match state.generations().get(human - 1) {
            Some(generation) => generation.best(),
            None => {
                eprintln!("error: generation {human} is unavailable; checkpoint contains {} completed generations", state.generations().len());
                return ExitCode::from(2);
            }
        },
    };
    write_stdout_line(&format!(
        "Benchmark started: depth {}, openings {}, random genomes {}",
        command.config.search_depth,
        command.config.opening_count,
        command.config.random_genome_count
    ));
    let mut observer = |control: &blocky_evolution::benchmark::ControlResult| {
        let label = match control.opponent_index {
            Some(index) => format!("random-genome-{index}"),
            None => "random-legal".to_owned(),
        };
        write_stdout_line(&format!(
            "Benchmark control complete: {label}, candidate {}, opponent {}",
            control.candidate_score_half_points, control.opponent_score_half_points
        ));
    };
    let report = match blocky_evolution::benchmark::run_benchmark_with_observer(
        candidate.individual().genome(),
        &command.config,
        command.workers,
        &mut observer,
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("error: benchmark failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = write_benchmark_report(
        &command.report,
        training_seed,
        &command.selector,
        candidate,
        &report,
    ) {
        eprintln!("error: could not export benchmark report: {error}");
        return ExitCode::FAILURE;
    }
    write_stdout_line("Benchmark complete");
    ExitCode::SUCCESS
}

fn run_validate(command: ValidateCommand) -> ExitCode {
    let (evolution_config, state) = match read_checkpoint_unchecked_config(&command.checkpoint) {
        Ok(data) => data,
        Err(error) => {
            eprintln!("error: could not read checkpoint: {error}");
            return ExitCode::from(2);
        }
    };
    let training_seed = evolution_config.training().master_seed();
    if training_seed == command.validation.master_seed() {
        eprintln!("error: training and validation seeds must be different");
        return ExitCode::from(2);
    }
    let candidate = match &command.selector {
        CandidateSelector::BestEver => state.best_ever(),
        CandidateSelector::Generation(human) => match state.generations().get(human - 1) {
            Some(generation) => generation.best(),
            None => {
                eprintln!(
                    "error: generation {human} is unavailable; checkpoint contains {} completed generations",
                    state.generations().len()
                );
                return ExitCode::from(2);
            }
        },
    };
    let mut validator = ChampionValidator::production_parallel(
        command.validation,
        command.workers,
        Box::new(ConsoleProgressObserver::default()),
    );
    let validation = match validator.validate(candidate.individual().genome()) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("error: validation failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = write_validation_report(
        &command.report,
        training_seed,
        &command.selector,
        candidate,
        &validation,
    ) {
        eprintln!("error: could not export validation report: {error}");
        return ExitCode::FAILURE;
    }
    write_stdout_line(&format!(
        "Validation complete: candidate {}, reference {}",
        validation.candidate_score.0, validation.reference_score.0
    ));
    ExitCode::SUCCESS
}

fn run_train(command: TrainCommand) -> ExitCode {
    if command.evolution.training().master_seed() == command.validation.master_seed() {
        eprintln!("error: training and validation seeds must be different");
        return ExitCode::from(2);
    }
    let resumed = match command
        .resume
        .as_deref()
        .map(|path| read_checkpoint(path, &command.evolution))
        .transpose()
    {
        Ok(state) => state,
        Err(error) => {
            eprintln!("error: could not resume training: {error}");
            return ExitCode::from(2);
        }
    };
    let checkpoint_path = command
        .checkpoint
        .clone()
        .or_else(|| command.resume.clone());
    let total_generations = command.evolution.generations();
    let frequency = command.checkpoint_every;
    let evolution_config = command.evolution.clone();
    let mut trainer = EvolutionEngine::with_observer(
        command.evolution,
        SelfPlayPopulationEvaluator::parallel(ProductionGameRunner, command.workers),
        Box::new(ConsoleProgressObserver::default()),
    );
    let save = |state: &blocky_evolution::evolution::EvolutionState| {
        let should_save = state.next_generation().is_multiple_of(frequency)
            || state.next_generation() == total_generations;
        if should_save {
            if let Some(path) = checkpoint_path.as_deref() {
                write_checkpoint(path, &evolution_config, state)
                    .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)?;
                write_stdout_line(&format!(
                    "Checkpoint saved: generation {}",
                    state.next_generation()
                ));
            }
        }
        Ok(())
    };
    let evolution = match resumed {
        Some(state) => trainer.run_resuming(state, save),
        None => trainer.run_with_checkpoints(save),
    };
    let evolution = match evolution {
        Ok(result) => result,
        Err(error) => {
            eprintln!("error: training failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    if command.training_only {
        write_stdout_line(&format!(
            "Training complete: {} generations; validation skipped",
            evolution.generations().len()
        ));
        return ExitCode::SUCCESS;
    }
    let mut validator = ChampionValidator::production_parallel(
        command.validation,
        command.workers,
        Box::new(ConsoleProgressObserver::default()),
    );
    let validation = match validator.validate(evolution.best_ever().individual().genome()) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("error: validation failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report = ExperimentReport::new(evolution, validation);
    if let Some(path) = command.report.as_deref() {
        if let Err(error) = write_experiment_report(path, &evolution_config, &report) {
            eprintln!("error: could not export report: {error}");
            return ExitCode::FAILURE;
        }
    }
    print!("{}", render_summary(&report));
    use std::io::Write;
    let _ = std::io::stdout().flush();
    ExitCode::SUCCESS
}
