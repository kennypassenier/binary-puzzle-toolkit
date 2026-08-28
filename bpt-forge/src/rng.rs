//! M20/AR29: every puzzle's stream is derived from (batch seed, index,
//! attempt), never from shared mutable state, so a puzzle is identical
//! whether it was generated sequentially or on any core, and a single
//! failing puzzle can be regenerated on its own.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Derive the stream for one generation attempt.
///
/// Deriving rather than splitting a shared generator is what makes a
/// batch reproducible regardless of how it was scheduled: puzzle 7 of
/// seed 42 is the same puzzle whether it ran first, last, or on another
/// core, and re-running just that one needs no replay of the others.
pub fn stream(batch_seed: u64, index: u64, attempt: u64) -> ChaCha8Rng {
    // Mixed with odd constants so that neighbouring (index, attempt)
    // pairs do not produce neighbouring streams.
    let derived = batch_seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(index.wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(attempt.wrapping_mul(0x94D0_49BB_1331_11EB));
    ChaCha8Rng::seed_from_u64(derived)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    fn first_words(mut rng: ChaCha8Rng) -> [u32; 4] {
        [
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
        ]
    }

    #[test]
    fn m20_the_same_coordinates_give_the_same_stream() {
        assert_eq!(first_words(stream(42, 7, 0)), first_words(stream(42, 7, 0)));
    }

    #[test]
    fn m20_neighbouring_coordinates_give_unrelated_streams() {
        let a = first_words(stream(42, 7, 0));
        for (seed, index, attempt) in [(43, 7, 0), (42, 8, 0), (42, 7, 1)] {
            assert_ne!(
                a,
                first_words(stream(seed, index, attempt)),
                "({seed},{index},{attempt}) must not echo (42,7,0)"
            );
        }
    }

    #[test]
    fn m20_a_single_puzzle_regenerates_without_replaying_the_batch() {
        // Puzzle 5 is identical whether or not 0..4 were drawn first.
        let direct = first_words(stream(1234, 5, 0));
        for i in 0..5 {
            let _ = first_words(stream(1234, i, 0));
        }
        assert_eq!(direct, first_words(stream(1234, 5, 0)));
    }
}
