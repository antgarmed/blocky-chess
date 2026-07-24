//! Validated hyperparameters shared by the training components.

use std::{error::Error, fmt, ops::RangeInclusive};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainingConfig {
    search_depth: usize,
    max_game_plies: usize,
    master_seed: u64,
    opening_plies: RangeInclusive<usize>,
    max_opening_attempts: usize,
}

impl TrainingConfig {
    pub fn new(
        search_depth: usize,
        max_game_plies: usize,
        master_seed: u64,
        opening_plies: RangeInclusive<usize>,
        max_opening_attempts: usize,
    ) -> Result<Self, TrainingConfigError> {
        if search_depth == 0 {
            return Err(TrainingConfigError::ZeroSearchDepth);
        }
        if max_game_plies == 0 {
            return Err(TrainingConfigError::ZeroMaxGamePlies);
        }
        if opening_plies.is_empty() {
            return Err(TrainingConfigError::EmptyOpeningRange);
        }
        if !opening_plies.clone().any(|plies| plies % 2 == 0) {
            return Err(TrainingConfigError::OpeningRangeHasNoEvenLength);
        }
        if max_opening_attempts == 0 {
            return Err(TrainingConfigError::ZeroOpeningAttempts);
        }
        Ok(Self {
            search_depth,
            max_game_plies,
            master_seed,
            opening_plies,
            max_opening_attempts,
        })
    }

    pub const fn search_depth(&self) -> usize {
        self.search_depth
    }

    pub const fn max_game_plies(&self) -> usize {
        self.max_game_plies
    }

    pub const fn master_seed(&self) -> u64 {
        self.master_seed
    }

    pub const fn opening_plies(&self) -> &RangeInclusive<usize> {
        &self.opening_plies
    }

    pub const fn max_opening_attempts(&self) -> usize {
        self.max_opening_attempts
    }

    pub(crate) fn with_master_seed(&self, master_seed: u64) -> Self {
        let mut config = self.clone();
        config.master_seed = master_seed;
        config
    }
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            search_depth: 4,
            max_game_plies: 200,
            master_seed: 0,
            opening_plies: 4..=10,
            max_opening_attempts: 100,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainingConfigError {
    ZeroSearchDepth,
    ZeroMaxGamePlies,
    EmptyOpeningRange,
    OpeningRangeHasNoEvenLength,
    ZeroOpeningAttempts,
}

impl fmt::Display for TrainingConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSearchDepth => f.write_str("search depth must be greater than zero"),
            Self::ZeroMaxGamePlies => f.write_str("maximum game plies must be greater than zero"),
            Self::EmptyOpeningRange => f.write_str("opening plies range must not be empty"),
            Self::OpeningRangeHasNoEvenLength => {
                f.write_str("opening plies range must contain an even length")
            }
            Self::ZeroOpeningAttempts => {
                f.write_str("maximum opening attempts must be greater than zero")
            }
        }
    }
}

impl Error for TrainingConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_independent_hyperparameters() {
        assert_eq!(
            TrainingConfig::new(0, 100, 1, 4..=8, 10),
            Err(TrainingConfigError::ZeroSearchDepth)
        );
        assert_eq!(
            TrainingConfig::new(4, 0, 1, 4..=8, 10),
            Err(TrainingConfigError::ZeroMaxGamePlies)
        );
        let invalid_start = 8;
        let invalid_end = 4;
        assert_eq!(
            TrainingConfig::new(4, 100, 1, invalid_start..=invalid_end, 10),
            Err(TrainingConfigError::EmptyOpeningRange)
        );
        assert_eq!(
            TrainingConfig::new(4, 100, 1, 5..=5, 10),
            Err(TrainingConfigError::OpeningRangeHasNoEvenLength)
        );
        assert_eq!(
            TrainingConfig::new(4, 100, 1, 4..=8, 0),
            Err(TrainingConfigError::ZeroOpeningAttempts)
        );
    }

    #[test]
    fn defaults_are_safe_and_use_the_proposed_search_depth() {
        let config = TrainingConfig::default();
        assert_eq!(config.search_depth(), 4);
        assert_eq!(config.max_game_plies(), 200);
        assert_eq!(config.opening_plies(), &(4..=10));
        assert_eq!(config.max_opening_attempts(), 100);
    }
}
