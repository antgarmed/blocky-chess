use std::{env, process::ExitCode};

use blocky_evolution::{
    cli::{render_summary, Command, ConsoleProgressObserver, TrainCommand, HELP},
    experiment::ProductionExperimentService,
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
    let mut service = match ProductionExperimentService::production_with_observers(
        command.evolution,
        command.validation,
        Box::new(ConsoleProgressObserver),
        Box::new(ConsoleProgressObserver),
    ) {
        Ok(service) => service,
        Err(error) => {
            eprintln!("error: invalid experiment configuration: {error}");
            return ExitCode::from(2);
        }
    };
    match service.run() {
        Ok(report) => {
            print!("{}", render_summary(&report));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: experiment failed: {error}");
            ExitCode::FAILURE
        }
    }
}
