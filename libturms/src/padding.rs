//! Random crypto-secure padding.

// Minimum required by specification.
const MIN_LENGTH: usize = 1000; // ~1kB.

/// Padding structure.
#[derive(Debug, Clone)]
pub(crate) struct Padding;

impl Padding {
    fn bucket_size(len: usize) -> usize {
        match len {
            0..=MIN_LENGTH => MIN_LENGTH,
            1001..8192 => len.div_ceil(1024) * 1024,
            _ => len,
        }
    }

    /// Fill an entry with bunch of paddings using ISO/IEC 7816-4.
    pub fn pad(data: impl AsRef<[u8]>) -> Vec<u8> {
        let data = data.as_ref();

        let target_len = Self::bucket_size(data.len());
        let mut out = Vec::with_capacity(target_len);

        out.extend_from_slice(data);
        out.push(0x80);

        if out.len() < MIN_LENGTH {
            out.resize(MIN_LENGTH, 0x00);
        }

        out
    }

    /// Remove zero padding at the end of data using ISO/IEC 7816-4.
    pub fn unpad(mut data: Vec<u8>) -> Vec<u8> {
        // Scan from the end.
        while let Some(&last) = data.last() {
            match last {
                0x00 => {
                    data.pop();
                },
                0x80 => {
                    data.pop();
                    return data;
                },
                _ => break,
            }
        }

        data
    }
}
