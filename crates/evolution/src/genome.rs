use std::{error::Error, fmt};

use blocky_chess::EvaluationConfig;

/// Number of independent coefficients in an evaluation genome.
pub const GENE_COUNT: usize = 12;

/// Global scale used when quantizing canonical genes for the integer evaluator.
///
/// A canonical genome has a largest coefficient of `1.0`. Mapping that value
/// to 4,000 retains useful precision for small coefficients while keeping even
/// a pathological mobility-only genome below the engine's reserved mate score
/// for the maximum number of legal moves in a chess position.
pub const EVALUATION_QUANTIZATION_SCALE: i64 = 4_000;

/// The global mobility weight used by configurations produced from a genome.
///
/// The evaluator calculates `global * piece / 100`. Setting the global value
/// to 100 makes each per-piece value the effective mobility coefficient and
/// removes the redundant second scale from the chromosome.
pub const EFFECTIVE_MOBILITY_WEIGHT: i64 = 100;

/// Stable position of a coefficient in [`Genome`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Gene {
    PawnMaterial = 0,
    KnightMaterial = 1,
    BishopMaterial = 2,
    RookMaterial = 3,
    QueenMaterial = 4,
    PawnMobility = 5,
    KnightMobility = 6,
    BishopMobility = 7,
    RookMobility = 8,
    QueenMobility = 9,
    KingMobility = 10,
    KingSafety = 11,
}

impl Gene {
    pub const ALL: [Self; GENE_COUNT] = [
        Self::PawnMaterial,
        Self::KnightMaterial,
        Self::BishopMaterial,
        Self::RookMaterial,
        Self::QueenMaterial,
        Self::PawnMobility,
        Self::KnightMobility,
        Self::BishopMobility,
        Self::RookMobility,
        Self::QueenMobility,
        Self::KingMobility,
        Self::KingSafety,
    ];
}

/// A scale-independent chromosome for the static evaluator.
///
/// All coefficients are finite and non-negative, at least one is positive,
/// and the largest is always exactly `1.0`. This max normalization makes
/// proportional chromosomes canonical without choosing a particular chess
/// feature (such as a pawn) as the unit.
#[derive(Clone, Debug, PartialEq)]
pub struct Genome {
    genes: [f64; GENE_COUNT],
}

impl Genome {
    /// Creates a canonical genome from arbitrary proportional coefficients.
    pub fn new(mut genes: [f64; GENE_COUNT]) -> Result<Self, GenomeError> {
        for (index, value) in genes.iter().copied().enumerate() {
            let gene = Gene::ALL[index];
            if !value.is_finite() {
                return Err(GenomeError::NonFiniteGene { gene, value });
            }
            if value < 0.0 {
                return Err(GenomeError::NegativeGene { gene, value });
            }
        }

        let maximum = genes.iter().copied().fold(0.0, f64::max);
        if maximum == 0.0 {
            return Err(GenomeError::AllZero);
        }

        for value in &mut genes {
            *value /= maximum;
        }

        Ok(Self { genes })
    }

    /// Returns the canonical coefficients in [`Gene`] order.
    pub fn genes(&self) -> &[f64; GENE_COUNT] {
        &self.genes
    }

    /// Returns one canonical coefficient.
    pub fn gene(&self, gene: Gene) -> f64 {
        self.genes[gene as usize]
    }

    /// Produces the integer configuration consumed by Blocky Chess.
    ///
    /// Coefficients below half a quantization step become zero. A valid genome
    /// still cannot produce an all-zero configuration because its largest gene
    /// is canonicalized to `1.0`.
    pub fn to_evaluation_config(&self) -> EvaluationConfig {
        let quantized = self.genes.map(quantize);

        EvaluationConfig {
            pawn_value: quantized[Gene::PawnMaterial as usize],
            knight_value: quantized[Gene::KnightMaterial as usize],
            bishop_value: quantized[Gene::BishopMaterial as usize],
            rook_value: quantized[Gene::RookMaterial as usize],
            queen_value: quantized[Gene::QueenMaterial as usize],
            mobility_weight: EFFECTIVE_MOBILITY_WEIGHT,
            pawn_mobility_weight: quantized[Gene::PawnMobility as usize],
            knight_mobility_weight: quantized[Gene::KnightMobility as usize],
            bishop_mobility_weight: quantized[Gene::BishopMobility as usize],
            rook_mobility_weight: quantized[Gene::RookMobility as usize],
            queen_mobility_weight: quantized[Gene::QueenMobility as usize],
            king_mobility_weight: quantized[Gene::KingMobility as usize],
            king_safety_weight: quantized[Gene::KingSafety as usize],
        }
    }
}

impl Default for Genome {
    fn default() -> Self {
        // EvaluationConfig::default(), expressed as effective coefficients and
        // max-normalized by its queen value (900).
        Self {
            genes: [
                100.0 / 900.0,
                300.0 / 900.0,
                300.0 / 900.0,
                500.0 / 900.0,
                1.0,
                0.5 / 900.0,
                3.0 / 900.0,
                3.0 / 900.0,
                2.0 / 900.0,
                1.0 / 900.0,
                0.5 / 900.0,
                50.0 / 900.0,
            ],
        }
    }
}

impl TryFrom<&EvaluationConfig> for Genome {
    type Error = GenomeError;

    fn try_from(config: &EvaluationConfig) -> Result<Self, Self::Error> {
        validate_configuration(config)?;

        let global_mobility = config.mobility_weight as f64 / 100.0;
        Self::new([
            config.pawn_value as f64,
            config.knight_value as f64,
            config.bishop_value as f64,
            config.rook_value as f64,
            config.queen_value as f64,
            global_mobility * config.pawn_mobility_weight as f64,
            global_mobility * config.knight_mobility_weight as f64,
            global_mobility * config.bishop_mobility_weight as f64,
            global_mobility * config.rook_mobility_weight as f64,
            global_mobility * config.queen_mobility_weight as f64,
            global_mobility * config.king_mobility_weight as f64,
            config.king_safety_weight as f64,
        ])
    }
}

impl TryFrom<EvaluationConfig> for Genome {
    type Error = GenomeError;

    fn try_from(config: EvaluationConfig) -> Result<Self, Self::Error> {
        Self::try_from(&config)
    }
}

impl From<&Genome> for EvaluationConfig {
    fn from(genome: &Genome) -> Self {
        genome.to_evaluation_config()
    }
}

impl From<Genome> for EvaluationConfig {
    fn from(genome: Genome) -> Self {
        genome.to_evaluation_config()
    }
}

fn quantize(value: f64) -> i64 {
    (value * EVALUATION_QUANTIZATION_SCALE as f64).round() as i64
}

fn validate_configuration(config: &EvaluationConfig) -> Result<(), GenomeError> {
    let fields = [
        ("pawn_value", config.pawn_value),
        ("knight_value", config.knight_value),
        ("bishop_value", config.bishop_value),
        ("rook_value", config.rook_value),
        ("queen_value", config.queen_value),
        ("mobility_weight", config.mobility_weight),
        ("pawn_mobility_weight", config.pawn_mobility_weight),
        ("knight_mobility_weight", config.knight_mobility_weight),
        ("bishop_mobility_weight", config.bishop_mobility_weight),
        ("rook_mobility_weight", config.rook_mobility_weight),
        ("queen_mobility_weight", config.queen_mobility_weight),
        ("king_mobility_weight", config.king_mobility_weight),
        ("king_safety_weight", config.king_safety_weight),
    ];

    for (field, value) in fields {
        if value < 0 {
            return Err(GenomeError::NegativeConfigurationValue { field, value });
        }
    }

    Ok(())
}

/// Invalid input rejected while constructing a canonical genome.
#[derive(Clone, Debug, PartialEq)]
pub enum GenomeError {
    NonFiniteGene { gene: Gene, value: f64 },
    NegativeGene { gene: Gene, value: f64 },
    NegativeConfigurationValue { field: &'static str, value: i64 },
    AllZero,
}

impl fmt::Display for GenomeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteGene { gene, value } => {
                write!(formatter, "{gene:?} must be finite, got {value}")
            }
            Self::NegativeGene { gene, value } => {
                write!(formatter, "{gene:?} must be non-negative, got {value}")
            }
            Self::NegativeConfigurationValue { field, value } => {
                write!(formatter, "{field} must be non-negative, got {value}")
            }
            Self::AllZero => {
                formatter.write_str("a genome must contain at least one positive gene")
            }
        }
    }
}

impl Error for GenomeError {}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1.0e-12;

    #[test]
    fn gene_order_is_stable() {
        for (expected_index, gene) in Gene::ALL.into_iter().enumerate() {
            assert_eq!(gene as usize, expected_index);
        }
    }

    #[test]
    fn normalizes_by_the_largest_gene() {
        let genome =
            Genome::new([2.0, 4.0, 1.0, 0.0, 8.0, 2.0, 4.0, 1.0, 0.0, 8.0, 2.0, 4.0]).unwrap();

        assert_eq!(
            genome.genes(),
            &[0.25, 0.5, 0.125, 0.0, 1.0, 0.25, 0.5, 0.125, 0.0, 1.0, 0.25, 0.5]
        );
    }

    #[test]
    fn proportional_inputs_have_the_same_canonical_form() {
        let first = Genome::new([1.0; GENE_COUNT]).unwrap();
        let second = Genome::new([37.5; GENE_COUNT]).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn rejects_non_finite_genes() {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut genes = [1.0; GENE_COUNT];
            genes[Gene::BishopMobility as usize] = invalid;

            assert!(matches!(
                Genome::new(genes),
                Err(GenomeError::NonFiniteGene {
                    gene: Gene::BishopMobility,
                    value
                }) if value.to_bits() == invalid.to_bits()
            ));
        }
    }

    #[test]
    fn rejects_negative_genes_and_all_zero_genomes() {
        let mut genes = [1.0; GENE_COUNT];
        genes[Gene::KingSafety as usize] = -0.01;

        assert_eq!(
            Genome::new(genes),
            Err(GenomeError::NegativeGene {
                gene: Gene::KingSafety,
                value: -0.01,
            })
        );
        assert_eq!(Genome::new([0.0; GENE_COUNT]), Err(GenomeError::AllZero));
    }

    #[test]
    fn reads_effective_genes_from_configuration_in_documented_order() {
        let config = EvaluationConfig {
            pawn_value: 10,
            knight_value: 20,
            bishop_value: 30,
            rook_value: 40,
            queen_value: 50,
            mobility_weight: 25,
            pawn_mobility_weight: 24,
            knight_mobility_weight: 28,
            bishop_mobility_weight: 32,
            rook_mobility_weight: 36,
            queen_mobility_weight: 40,
            king_mobility_weight: 44,
            king_safety_weight: 12,
        };

        let genome = Genome::try_from(config).unwrap();
        let expected = [
            10.0, 20.0, 30.0, 40.0, 50.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ]
        .map(|value| value / 50.0);

        assert_eq!(genome.genes(), &expected);
    }

    #[test]
    fn writes_genes_to_configuration_in_documented_order() {
        let config = Genome::new([
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ])
        .unwrap()
        .to_evaluation_config();

        assert_eq!(config.pawn_value, 333);
        assert_eq!(config.knight_value, 667);
        assert_eq!(config.bishop_value, 1_000);
        assert_eq!(config.rook_value, 1_333);
        assert_eq!(config.queen_value, 1_667);
        assert_eq!(config.mobility_weight, EFFECTIVE_MOBILITY_WEIGHT);
        assert_eq!(config.pawn_mobility_weight, 2_000);
        assert_eq!(config.knight_mobility_weight, 2_333);
        assert_eq!(config.bishop_mobility_weight, 2_667);
        assert_eq!(config.rook_mobility_weight, 3_000);
        assert_eq!(config.queen_mobility_weight, 3_333);
        assert_eq!(config.king_mobility_weight, 3_667);
        assert_eq!(config.king_safety_weight, 4_000);
    }

    #[test]
    fn rejects_negative_configuration_fields_before_combining_mobility_scales() {
        let negative_global = EvaluationConfig {
            mobility_weight: -10,
            pawn_mobility_weight: -5,
            ..EvaluationConfig::default()
        };
        assert_eq!(
            Genome::try_from(negative_global),
            Err(GenomeError::NegativeConfigurationValue {
                field: "mobility_weight",
                value: -10,
            })
        );

        let negative_piece = EvaluationConfig {
            bishop_mobility_weight: -1,
            ..EvaluationConfig::default()
        };
        assert_eq!(
            Genome::try_from(negative_piece),
            Err(GenomeError::NegativeConfigurationValue {
                field: "bishop_mobility_weight",
                value: -1,
            })
        );
    }

    #[test]
    fn rejects_an_all_zero_configuration() {
        let zero = EvaluationConfig {
            pawn_value: 0,
            knight_value: 0,
            bishop_value: 0,
            rook_value: 0,
            queen_value: 0,
            mobility_weight: 0,
            pawn_mobility_weight: 0,
            knight_mobility_weight: 0,
            bishop_mobility_weight: 0,
            rook_mobility_weight: 0,
            queen_mobility_weight: 0,
            king_mobility_weight: 0,
            king_safety_weight: 0,
        };

        assert_eq!(Genome::try_from(zero), Err(GenomeError::AllZero));
    }

    #[test]
    fn default_genome_matches_default_evaluation_configuration() {
        let from_config = Genome::try_from(EvaluationConfig::default()).unwrap();

        for gene in Gene::ALL {
            assert!((Genome::default().gene(gene) - from_config.gene(gene)).abs() < EPSILON);
        }
    }

    #[test]
    fn conversion_uses_effective_mobility_scale_and_never_produces_all_zero() {
        let mut genes = [0.0; GENE_COUNT];
        genes[Gene::PawnMobility as usize] = f64::MIN_POSITIVE;
        let config = Genome::new(genes).unwrap().to_evaluation_config();

        assert_eq!(config.mobility_weight, EFFECTIVE_MOBILITY_WEIGHT);
        assert_eq!(config.pawn_mobility_weight, EVALUATION_QUANTIZATION_SCALE);
        assert_ne!(
            [
                config.pawn_value,
                config.knight_value,
                config.bishop_value,
                config.rook_value,
                config.queen_value,
                config.pawn_mobility_weight,
                config.knight_mobility_weight,
                config.bishop_mobility_weight,
                config.rook_mobility_weight,
                config.queen_mobility_weight,
                config.king_mobility_weight,
                config.king_safety_weight,
            ],
            [0; GENE_COUNT]
        );
    }

    #[test]
    fn sub_resolution_genes_quantize_to_zero_without_zeroing_the_configuration() {
        let mut genes = [0.0; GENE_COUNT];
        genes[Gene::QueenMaterial as usize] = 1.0;
        genes[Gene::KingMobility as usize] = f64::MIN_POSITIVE;
        let config = Genome::new(genes).unwrap().to_evaluation_config();

        assert_eq!(config.queen_value, EVALUATION_QUANTIZATION_SCALE);
        assert_eq!(config.king_mobility_weight, 0);
    }

    #[test]
    fn configuration_round_trip_preserves_proportions_within_quantization_error() {
        let original = Genome::new([
            100.0, 317.0, 331.0, 503.0, 947.0, 2.0, 11.0, 13.0, 17.0, 19.0, 3.0, 71.0,
        ])
        .unwrap();
        let config = original.to_evaluation_config();
        let round_trip = Genome::try_from(config).unwrap();
        let tolerance = 0.5 / EVALUATION_QUANTIZATION_SCALE as f64 + EPSILON;

        for gene in Gene::ALL {
            assert!(
                (original.gene(gene) - round_trip.gene(gene)).abs() <= tolerance,
                "{gene:?}: {} != {}",
                original.gene(gene),
                round_trip.gene(gene)
            );
        }
    }
}
