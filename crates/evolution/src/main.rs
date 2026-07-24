use std::{env, process::ExitCode};

use blocky_evolution::{
    cli::{render_summary, Command, ConsoleProgressObserver, TrainCommand, HELP},
    encounter::ProductionGameRunner,
    evolution::{EvolutionEngine, SelfPlayPopulationEvaluator},
    experiment::ExperimentReport,
    persistence::{read_checkpoint, write_checkpoint, write_experiment_report},
    validation::ChampionValidator,
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
    }
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
        SelfPlayPopulationEvaluator::new(ProductionGameRunner),
        Box::new(ConsoleProgressObserver),
    );
    let save = |state: &blocky_evolution::evolution::EvolutionState| {
        let should_save = state.next_generation().is_multiple_of(frequency)
            || state.next_generation() == total_generations;
        if should_save {
            if let Some(path) = checkpoint_path.as_deref() {
                write_checkpoint(path, &evolution_config, state)
                    .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)?;
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
    let mut validator = ChampionValidator::with_observer(
        command.validation,
        ProductionGameRunner,
        Box::new(ConsoleProgressObserver),
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
    ExitCode::SUCCESS
}
