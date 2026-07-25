use crate::evaluation::EvaluationConfig;
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("cannot read checkpoint: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid checkpoint JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("individual {0} was not found")]
    NotFound(u64),
    #[error("individual {0} has invalid genes")]
    InvalidGenes(u64),
}

#[derive(Deserialize)]
struct Checkpoint {
    state: State,
}
#[derive(Deserialize)]
struct State {
    population: Vec<Individual>,
    generations: Vec<Generation>,
    best_ever: Evaluated,
}
#[derive(Deserialize)]
struct Generation {
    ranked: Vec<Evaluated>,
}
#[derive(Deserialize)]
struct Evaluated {
    individual: Individual,
}
#[derive(Deserialize)]
struct Individual {
    id: u64,
    genes: [f64; 12],
}

pub fn load_individual(path: impl AsRef<Path>, id: u64) -> Result<EvaluationConfig, ProfileError> {
    let checkpoint: Checkpoint = serde_json::from_slice(&fs::read(path)?)?;
    let mut found = checkpoint
        .state
        .population
        .into_iter()
        .find(|i| i.id == id)
        .or_else(|| {
            (checkpoint.state.best_ever.individual.id == id)
                .then_some(checkpoint.state.best_ever.individual)
        });
    if found.is_none() {
        found = checkpoint
            .state
            .generations
            .into_iter()
            .flat_map(|g| g.ranked)
            .map(|e| e.individual)
            .find(|i| i.id == id);
    }
    let individual = found.ok_or(ProfileError::NotFound(id))?;
    EvaluationConfig::from_normalized_genes(individual.genes).ok_or(ProfileError::InvalidGenes(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn loads_an_individual_from_generation_history() {
        let path = std::env::temp_dir().join(format!("blocky-profile-{}.json", std::process::id()));
        let mut file = std::fs::File::create(&path).unwrap();
        write!(file, r#"{{"state":{{"population":[],"generations":[{{"ranked":[{{"individual":{{"id":7,"genes":[0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2]}}}}]}}],"best_ever":{{"individual":{{"id":9,"genes":[1.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]}}}}}}}}"#).unwrap();
        let config = load_individual(&path, 7).unwrap();
        assert_eq!(config.queen_value, 2_000);
        assert_eq!(config.mobility_weight, 100);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn reports_unknown_individual() {
        let error = load_individual("missing-checkpoint.json", 7).unwrap_err();
        assert!(matches!(error, ProfileError::Io(_)));
    }
}
