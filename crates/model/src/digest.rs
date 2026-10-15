// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Full content identities and incremental SHA-256, independently implemented from
//! FIPS PUB 180-4 (August 2015), sections 4.1.2, 4.2.2, 5 and 6.2.
//! No filesystem, platform API, allocation or third-party implementation is used.

use core::{fmt, str::FromStr};

/// A SHA-256 identity of exact bytes; text is always 64 lowercase hexadecimal digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentDigest([u8; 32]);

impl ContentDigest {
    /// Constructs an identity from all 32 raw digest bytes (not a message).
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The complete raw digest, for binary protocols such as F05 attachments.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// An invalid digest representation; paths, URIs and shortened forms are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidDigest;

impl fmt::Display for InvalidDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("digest must contain exactly 64 lowercase hexadecimal digits")
    }
}

impl core::error::Error for InvalidDigest {}

impl FromStr for ContentDigest {
    type Err = InvalidDigest;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() != 64 {
            return Err(InvalidDigest);
        }
        let mut bytes = [0; 32];
        for (slot, pair) in bytes.iter_mut().zip(text.as_bytes().chunks_exact(2)) {
            let mut value = 0;
            for c in pair {
                let digit = match c {
                    b'0'..=b'9' => c - b'0',
                    b'a'..=b'f' => c - b'a' + 10,
                    _ => return Err(InvalidDigest),
                };
                value = (value << 4) | digit;
            }
            *slot = value;
        }
        Ok(Self(bytes))
    }
}

/// The message exceeds SHA-256's less-than-2^64-bit length domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthExceeded;

impl fmt::Display for LengthExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SHA-256 message length exceeds its bit-counter limit")
    }
}

impl core::error::Error for LengthExceeded {}

/// Largest whole-byte SHA-256 message length; padding is not counted.
pub const MAX_MESSAGE_BYTES: u64 = u64::MAX / 8;

/// Incremental SHA-256 with a checked byte counter and a fixed 64-byte buffer.
#[derive(Clone)]
pub struct Sha256 {
    state: [u32; 8],
    block: [u8; 64],
    used: usize,
    length: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// Initializes the eight words specified by FIPS 180-4 section 5.3.3.
    pub const fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            block: [0; 64],
            used: 0,
            length: 0,
        }
    }

    /// Adds bytes. A length failure leaves this hasher unchanged.
    pub fn update(&mut self, bytes: &[u8]) -> Result<(), LengthExceeded> {
        let added = u64::try_from(bytes.len()).map_err(|_| LengthExceeded)?;
        let length = self
            .length
            .checked_add(added)
            .filter(|n| *n <= MAX_MESSAGE_BYTES)
            .ok_or(LengthExceeded)?;
        for byte in bytes {
            self.push(*byte);
        }
        self.length = length;
        Ok(())
    }

    // Only internal, bounded positions reach this buffer; get_mut also avoids panics.
    fn push(&mut self, byte: u8) {
        if let Some(slot) = self.block.get_mut(self.used) {
            *slot = byte;
        }
        self.used += 1;
        if self.used == 64 {
            compress(&mut self.state, &self.block);
            self.used = 0;
        }
    }

    /// Consumes the hasher, appends FIPS section 5.1.1 padding and returns all 256 bits.
    pub fn finalize(mut self) -> ContentDigest {
        let bits = self.length * 8; // update enforces MAX_MESSAGE_BYTES.
        self.push(0x80);
        while self.used != 56 {
            self.push(0);
        }
        for byte in bits.to_be_bytes() {
            self.push(byte);
        }
        let mut bytes = [0; 32];
        for (out, word) in bytes.chunks_exact_mut(4).zip(self.state) {
            out.copy_from_slice(&word.to_be_bytes());
        }
        ContentDigest(bytes)
    }

    /// Hashes one complete message, enforcing the same length domain as [`Self::update`].
    pub fn digest(bytes: &[u8]) -> Result<ContentDigest, LengthExceeded> {
        let mut hash = Self::new();
        hash.update(bytes)?;
        Ok(hash.finalize())
    }
}

// FIPS 180-4 section 4.2.2: first 32 bits of fractional cube roots of the first 64 primes.
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut words = [0u32; 64];
    for (out, bytes) in words.iter_mut().zip(block.chunks_exact(4)) {
        *out = bytes.iter().fold(0u32, |n, b| (n << 8) | u32::from(*b));
    }
    // All positions below derive solely from the fixed SHA-256 schedule, not input.
    for t in 16usize..64 {
        let at = |i| words.get(i).copied().unwrap_or(0);
        let x = at(t - 15);
        let y = at(t - 2);
        let s0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
        let s1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
        let value = at(t - 16)
            .wrapping_add(s0)
            .wrapping_add(at(t - 7))
            .wrapping_add(s1);
        if let Some(out) = words.get_mut(t) {
            *out = value;
        }
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for (word, k) in words.into_iter().zip(K) {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(k)
            .wrapping_add(word);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        (h, g, f, e, d, c, b, a) = (g, f, e, d.wrapping_add(t1), c, b, a, t1.wrapping_add(t2));
    }
    for (out, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *out = out.wrapping_add(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nist_examples() {
        // NIST SHA-256 examples: empty, abc, the 448-bit message and a million 'a's.
        for (message, expected) in [
            (
                b"".as_slice(),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc".as_slice(),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".as_slice(),
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
        ] {
            assert_eq!(Sha256::digest(message).unwrap().to_string(), expected);
        }
        assert_eq!(
            Sha256::digest(&vec![b'a'; 1_000_000]).unwrap().to_string(),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn padding_boundaries_and_all_chunk_sizes() {
        // Independent oracle: Windows .NET System.Security.Cryptography.SHA256,
        // computed for bytes 0..n (mod 251), not the fixture tool's implementation.
        for (len, expected) in [
            (
                55,
                "463eb28e72f82e0a96c0a4cc53690c571281131f672aa229e0d45ae59b598b59",
            ),
            (
                56,
                "da2ae4d6b36748f2a318f23e7ab1dfdf45acdc9d049bd80e59de82a60895f562",
            ),
            (
                63,
                "29af2686fd53374a36b0846694cc342177e428d1647515f078784d69cdb9e488",
            ),
            (
                64,
                "fdeab9acf3710362bd2658cdc9a29e8f9c757fcf9811603a8c447cd1d9151108",
            ),
            (
                65,
                "4bfd2c8b6f1eec7a2afeb48b934ee4b2694182027e6d0fc075074f2fabb31781",
            ),
            (
                119,
                "da18797ed7c3a777f0847f429724a2d8cd5138e6ed2895c3fa1a6d39d18f7ec6",
            ),
            (
                120,
                "f52b23db1fbb6ded89ef42a23ce0c8922c45f25c50b568a93bf1c075420bbb7c",
            ),
        ] {
            let message: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
            let digest = Sha256::digest(&message).unwrap();
            assert_eq!(digest.to_string(), expected, "length {len}");
            for size in 1..=len + 1 {
                let mut hash = Sha256::new();
                for chunk in message.chunks(size) {
                    hash.update(&[]).unwrap();
                    hash.update(chunk).unwrap();
                }
                assert_eq!(hash.finalize(), digest, "length {len}, chunk size {size}");
            }
            for split in 0..=len {
                let mut hash = Sha256::default();
                hash.update(&message[..split]).unwrap();
                hash.update(&message[split..]).unwrap();
                assert_eq!(hash.finalize(), digest, "length {len}, split {split}");
            }
        }
    }

    #[test]
    fn parse_is_strict_and_round_trips_raw_bytes() {
        let text = "0123456789abcdef".repeat(4);
        let digest: ContentDigest = text.parse().unwrap();
        assert_eq!(digest.to_string(), text);
        assert_eq!(ContentDigest::from_bytes(*digest.as_bytes()), digest);
        for bad in [
            text.to_uppercase(),
            text[..63].to_owned(),
            format!("{text}0"),
            format!("g{}", &text[1..]),
            format!(" {text}"),
            format!("{text}\n"),
            format!(" {}", &text[1..]),
            "é".repeat(32),
            String::new(),
            "https://example.test/blob".to_owned(),
            "C:\\blob".to_owned(),
            "../blob".to_owned(),
        ] {
            assert_eq!(bad.parse::<ContentDigest>(), Err(InvalidDigest), "{bad:?}");
        }
    }

    #[test]
    fn bounded_length_failure_preserves_state() {
        let mut hash = Sha256::new();
        hash.length = MAX_MESSAGE_BYTES - 1;
        hash.update(&[0]).unwrap();
        let before = hash.clone().finalize();
        assert_eq!(hash.update(&[1]), Err(LengthExceeded));
        assert_eq!(hash.finalize(), before);
        let mut hash = Sha256::new();
        hash.length = u64::MAX;
        assert_eq!(hash.update(&[0]), Err(LengthExceeded));
    }
}
