//! Deterministic summaries of completed self-play games.

use crate::self_play::{DrawReason, GameOutcome, GameRecord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameObservation {
    pub outcome: GameOutcome,
    pub plies: usize,
}

impl From<&GameRecord> for GameObservation {
    fn from(record: &GameRecord) -> Self {
        Self {
            outcome: record.outcome,
            plies: record.moves.len(),
        }
    }
}

/// Aggregate chess metrics. Wall-clock measurements deliberately live outside
/// this type so reports remain reproducible across machines and worker counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameStatistics {
    pub games: usize,
    pub white_wins: usize,
    pub black_wins: usize,
    pub draws: usize,
    pub stalemates: usize,
    pub insufficient_material: usize,
    pub threefold_repetitions: usize,
    pub fifty_move_rule: usize,
    pub max_plies_draws: usize,
    pub total_plies: usize,
    pub minimum_plies: usize,
    pub median_plies: usize,
    pub p95_plies: usize,
    pub maximum_plies: usize,
}

impl GameStatistics {
    pub fn from_records<'a>(records: impl IntoIterator<Item = &'a GameRecord>) -> Self {
        let mut outcomes = OutcomeCounts::default();
        let mut plies = Vec::new();
        for record in records {
            outcomes.record(record.outcome);
            plies.push(record.moves.len());
        }
        Self::from_parts(outcomes, plies)
    }

    pub(crate) fn from_outcomes_and_plies(
        games: impl IntoIterator<Item = (GameOutcome, usize)>,
    ) -> Self {
        let mut outcomes = OutcomeCounts::default();
        let mut plies = Vec::new();
        for (outcome, game_plies) in games {
            outcomes.record(outcome);
            plies.push(game_plies);
        }
        Self::from_parts(outcomes, plies)
    }

    pub fn from_observations(games: impl IntoIterator<Item = GameObservation>) -> Self {
        Self::from_outcomes_and_plies(games.into_iter().map(|game| (game.outcome, game.plies)))
    }

    fn from_parts(outcomes: OutcomeCounts, mut plies: Vec<usize>) -> Self {
        plies.sort_unstable();
        let games = plies.len();
        let total_plies = plies.iter().sum();
        Self {
            games,
            white_wins: outcomes.white_wins,
            black_wins: outcomes.black_wins,
            draws: outcomes.draws,
            stalemates: outcomes.stalemates,
            insufficient_material: outcomes.insufficient_material,
            threefold_repetitions: outcomes.threefold_repetitions,
            fifty_move_rule: outcomes.fifty_move_rule,
            max_plies_draws: outcomes.max_plies_draws,
            total_plies,
            minimum_plies: plies.first().copied().unwrap_or(0),
            median_plies: percentile(&plies, 50),
            p95_plies: percentile(&plies, 95),
            maximum_plies: plies.last().copied().unwrap_or(0),
        }
    }

    pub fn mean_plies(self) -> f64 {
        if self.games == 0 {
            0.0
        } else {
            self.total_plies as f64 / self.games as f64
        }
    }
}

#[derive(Default)]
struct OutcomeCounts {
    white_wins: usize,
    black_wins: usize,
    draws: usize,
    stalemates: usize,
    insufficient_material: usize,
    threefold_repetitions: usize,
    fifty_move_rule: usize,
    max_plies_draws: usize,
}

impl OutcomeCounts {
    fn record(&mut self, outcome: GameOutcome) {
        match outcome {
            GameOutcome::WhiteWin => self.white_wins += 1,
            GameOutcome::BlackWin => self.black_wins += 1,
            GameOutcome::Draw(reason) => {
                self.draws += 1;
                match reason {
                    DrawReason::Stalemate => self.stalemates += 1,
                    DrawReason::InsufficientMaterial => self.insufficient_material += 1,
                    DrawReason::ThreefoldRepetition => self.threefold_repetitions += 1,
                    DrawReason::FiftyMoveRule => self.fifty_move_rule += 1,
                    DrawReason::MaxPlies => self.max_plies_draws += 1,
                }
            }
        }
    }
}

fn percentile(sorted: &[usize], percentile: usize) -> usize {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (percentile * sorted.len()).div_ceil(100);
    sorted[rank.saturating_sub(1)]
}

#[cfg(test)]
mod tests {
    use shakmaty::{Chess, Position};

    use super::*;

    fn record(outcome: GameOutcome, plies: usize) -> GameRecord {
        GameRecord {
            outcome,
            moves: Chess::default()
                .legal_moves()
                .into_iter()
                .cycle()
                .take(plies)
                .collect(),
            position_history: vec![Chess::default()],
            final_position: Chess::default(),
        }
    }

    #[test]
    fn summarizes_outcomes_draw_reasons_and_plies() {
        let records = [
            record(GameOutcome::WhiteWin, 10),
            record(GameOutcome::BlackWin, 20),
            record(GameOutcome::Draw(DrawReason::ThreefoldRepetition), 30),
            record(GameOutcome::Draw(DrawReason::MaxPlies), 40),
        ];

        let statistics = GameStatistics::from_records(&records);

        assert_eq!(statistics.games, 4);
        assert_eq!(statistics.white_wins, 1);
        assert_eq!(statistics.black_wins, 1);
        assert_eq!(statistics.draws, 2);
        assert_eq!(statistics.threefold_repetitions, 1);
        assert_eq!(statistics.max_plies_draws, 1);
        assert_eq!(statistics.total_plies, 100);
        assert_eq!(statistics.minimum_plies, 10);
        assert_eq!(statistics.median_plies, 20);
        assert_eq!(statistics.p95_plies, 40);
        assert_eq!(statistics.maximum_plies, 40);
        assert_eq!(statistics.mean_plies(), 25.0);
    }

    #[test]
    fn empty_statistics_are_well_defined() {
        assert_eq!(GameStatistics::default().mean_plies(), 0.0);
        assert_eq!(
            GameStatistics::from_records(std::iter::empty()),
            GameStatistics::default()
        );
    }
}
