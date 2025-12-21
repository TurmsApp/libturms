//! Random crypto-secure padding.

use rand::{RngCore, SeedableRng, TryRngCore};
use rand::rngs::OsRng;
use rand_chacha::ChaCha20Rng;
use error::Result;

// Numbers from specification.
const MIN_LENGTH: usize = 1000; // 1kB.
const PADDING_LENGTH: [usize; 2] = [0, 8192];

/// Padding structure.
#[derive(Debug, Clone)]
pub(crate) struct Padding {
    min_length: usize,
    padding_length: [usize; 2],
    fill_padding: u8,
}

impl Default for Padding {
    fn default() -> Self {
        Padding {
            min_length: MIN_LENGTH,
            padding_length: PADDING_LENGTH,
            fill_padding: 0, // adds lots of zeros.
        }
    }
}

impl Padding {
    /// Fill an entry with bunch of paddings.
    pub fn fill_zero(entry: impl AsRef<[u8]>) -> Result<Vec<u8>> {
        let config = Self::default();
        let data = entry.as_ref();
        let data_len = data.len();

        let mut seed = [0u8; 32];
        OsRng.try_fill_bytes(&mut seed)?;

        let mut rng = ChaCha20Rng::from_seed(seed);
        rng.fill_bytes(&mut data);

        let base_target = std::cmp::max(data_len, config.min_length);
        let total_size = base_target + data.len();

        let mut padded_data = Vec::with_capacity(total_size);
        padded_data.extend_from_slice(data);
        padded_data.resize(total_size, config.fill_padding);

        Ok(padded_data)
    }
}
