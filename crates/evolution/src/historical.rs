//! Deterministic historical champion league for zero-knowledge training.

use crate::{
    evolution::{EvaluatedIndividual, Individual},
    genome::Genome,
    openings::OpeningId,
    pairing::IndividualId,
    rng::derive_seed,
};

const SAMPLE_DOMAIN: u64 = 0x4849_5354_5341_4d50;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HistoricalConfig {
    weight_percent: u8,
    opponents: usize,
    opening_pairs: usize,
    insertion_cadence: usize,
    maximum_size: usize,
}

impl HistoricalConfig {
    pub fn new(
        weight_percent: u8,
        opponents: usize,
        opening_pairs: usize,
        insertion_cadence: usize,
        maximum_size: usize,
    ) -> Result<Self, HistoricalConfigError> {
        if weight_percent > 100 {
            return Err(HistoricalConfigError::WeightOutOfRange(weight_percent));
        }
        let dimensions = [opponents, opening_pairs, insertion_cadence, maximum_size];
        let enabled = weight_percent > 0;
        if (!enabled && dimensions.iter().any(|value| *value != 0))
            || (enabled && dimensions.contains(&0))
        {
            return Err(HistoricalConfigError::Inconsistent);
        }
        Ok(Self {
            weight_percent,
            opponents,
            opening_pairs,
            insertion_cadence,
            maximum_size,
        })
    }

    pub const fn enabled(self) -> bool {
        self.weight_percent > 0
    }
    pub const fn weight_percent(self) -> u8 {
        self.weight_percent
    }
    pub const fn opponents(self) -> usize {
        self.opponents
    }
    pub const fn opening_pairs(self) -> usize {
        self.opening_pairs
    }
    pub const fn insertion_cadence(self) -> usize {
        self.insertion_cadence
    }
    pub const fn maximum_size(self) -> usize {
        self.maximum_size
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoricalConfigError {
    WeightOutOfRange(u8),
    Inconsistent,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArchiveEntry {
    generation: usize,
    champion: Individual,
}

impl ArchiveEntry {
    pub fn new(generation: usize, champion: Individual) -> Self {
        Self {
            generation,
            champion,
        }
    }
    pub const fn generation(&self) -> usize {
        self.generation
    }
    pub const fn champion(&self) -> &Individual {
        &self.champion
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HistoricalArchive {
    entries: Vec<ArchiveEntry>,
}

impl HistoricalArchive {
    pub fn from_entries(entries: Vec<ArchiveEntry>) -> Result<Self, &'static str> {
        if entries
            .windows(2)
            .any(|pair| pair[0].generation >= pair[1].generation)
        {
            return Err("archive generations must be strictly increasing");
        }
        Ok(Self { entries })
    }
    pub fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }

    pub fn insert_champion(
        &mut self,
        generation: usize,
        champion: &EvaluatedIndividual,
        config: HistoricalConfig,
    ) {
        if !config.enabled() || !(generation + 1).is_multiple_of(config.insertion_cadence()) {
            return;
        }
        self.entries
            .push(ArchiveEntry::new(generation, champion.individual().clone()));
        while self.entries.len() > config.maximum_size() {
            let remove = least_temporally_useful(&self.entries);
            self.entries.remove(remove);
        }
    }

    pub fn sample(&self, count: usize, master_seed: u64, generation: usize) -> Vec<ArchiveEntry> {
        let mut ranked = self.entries.clone();
        ranked.sort_by_key(|entry| {
            (
                derive_seed(
                    master_seed,
                    SAMPLE_DOMAIN ^ generation as u64,
                    entry.generation as u64,
                ),
                entry.generation,
            )
        });
        ranked.truncate(count.min(ranked.len()));
        ranked.sort_by_key(ArchiveEntry::generation);
        ranked
    }
}

fn least_temporally_useful(entries: &[ArchiveEntry]) -> usize {
    if entries.len() <= 2 {
        return 0;
    }
    (1..entries.len() - 1)
        .max_by_key(|&removed| {
            let minimum_gap = entries
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != removed)
                .map(|(_, entry)| entry.generation)
                .collect::<Vec<_>>()
                .windows(2)
                .map(|pair| pair[1] - pair[0])
                .min()
                .unwrap_or(usize::MAX);
            (minimum_gap, std::cmp::Reverse(entries[removed].generation))
        })
        .expect("an overfull archive has an interior entry")
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HistoricalAudit {
    pub opponent_generations: Vec<usize>,
    pub opponent_ids: Vec<IndividualId>,
    pub opening_ids: Vec<OpeningId>,
    pub distinct_phenotypes: usize,
    pub archive_size_before: usize,
    pub archive_size_after: usize,
}

pub fn phenotype_fingerprint(genome: &Genome) -> [i64; 13] {
    let c = genome.to_evaluation_config();
    [
        c.pawn_value,
        c.knight_value,
        c.bishop_value,
        c.rook_value,
        c.queen_value,
        c.mobility_weight,
        c.pawn_mobility_weight,
        c.knight_mobility_weight,
        c.bishop_mobility_weight,
        c.rook_mobility_weight,
        c.queen_mobility_weight,
        c.king_mobility_weight,
        c.king_safety_weight,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{evolution::FitnessScore, pairing::Score};

    fn evaluated(generation: usize) -> EvaluatedIndividual {
        let mut genes = [0.1; 12];
        genes[generation % 12] = 1.0;
        EvaluatedIndividual::with_fitness(
            Individual::new(IndividualId(generation as u64), Genome::new(genes).unwrap()),
            FitnessScore::legacy(Score(0)),
        )
    }

    #[test]
    fn sampling_is_deterministic_and_partial_archives_normalize_the_request() {
        let config = HistoricalConfig::new(30, 5, 2, 1, 8).unwrap();
        let mut archive = HistoricalArchive::default();
        for generation in 0..3 {
            archive.insert_champion(generation, &evaluated(generation), config);
        }
        assert_eq!(archive.sample(5, 42, 9), archive.sample(5, 42, 9));
        assert_eq!(archive.sample(5, 42, 9).len(), 3);
    }

    #[test]
    fn bounded_retention_preserves_old_and_new_temporal_coverage() {
        let config = HistoricalConfig::new(30, 2, 1, 1, 3).unwrap();
        let mut archive = HistoricalArchive::default();
        for generation in 0..10 {
            archive.insert_champion(generation, &evaluated(generation), config);
        }
        let generations: Vec<_> = archive
            .entries()
            .iter()
            .map(ArchiveEntry::generation)
            .collect();
        assert_eq!(generations.first(), Some(&0));
        assert_eq!(generations.last(), Some(&9));
        assert_eq!(generations.len(), 3);
    }

    #[test]
    fn phenotype_fingerprint_uses_quantized_configuration() {
        let a = Genome::new([
            1.0, 0.100_01, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1,
        ])
        .unwrap();
        let b = Genome::new([
            1.0, 0.100_02, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1,
        ])
        .unwrap();
        assert_ne!(a, b);
        assert_eq!(phenotype_fingerprint(&a), phenotype_fingerprint(&b));
    }
}
