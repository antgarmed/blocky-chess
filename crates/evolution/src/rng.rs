//! Small, reproducible random streams used by training.

/// Object-safe random source so evolutionary operators can be replaced in
/// tests and by future strategies without making the engine generic over RNGs.
pub trait RandomSource {
    fn next_u64(&mut self) -> u64;

    fn unit_f64(&mut self) -> f64 {
        // Use the upper 53 bits and the midpoint of the represented interval.
        // The result is strictly between zero and one.
        ((self.next_u64() >> 11) as f64 + 0.5) / ((1_u64 << 53) as f64)
    }

    fn index(&mut self, length: usize) -> usize {
        assert!(length > 0, "cannot choose from an empty collection");
        // Rejection avoids modulo bias while retaining a fully stable stream.
        let threshold = u64::MAX - u64::MAX % length as u64;
        loop {
            let value = self.next_u64();
            if value < threshold {
                return (value % length as u64) as usize;
            }
        }
    }
}

/// Explicit SplitMix64 stream whose sequence is independent of dependencies.
#[derive(Clone, Debug)]
pub struct StableRng(u64);

impl StableRng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
}

impl RandomSource for StableRng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        mix(self.0)
    }
}

pub(crate) fn derive_seed(master: u64, stream: u64, attempt: u64) -> u64 {
    mix(master
        ^ stream.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ attempt.wrapping_mul(0xbf58_476d_1ce4_e5b9))
}

fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_are_reproducible_and_unit_values_are_open() {
        let mut first = StableRng::new(42);
        let mut second = StableRng::new(42);
        for _ in 0..20 {
            let a = first.next_u64();
            let b = second.next_u64();
            assert_eq!(a, b);
        }

        for _ in 0..20 {
            let value = first.unit_f64();
            assert!(value > 0.0 && value < 1.0);
        }
    }
}
