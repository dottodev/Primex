//! Hashing and digest verification helpers.

use crate::error::{PdxError, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex_encode(&sha256(data))
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(TABLE[(byte >> 4) as usize] as char);
        out.push(TABLE[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn hex_decode(input: &str) -> Result<Vec<u8>> {
    if input.len() % 2 != 0 {
        return Err(PdxError::bad_input("hex string has odd length")
            .expected("even number of hex digits", input.len().to_string()));
    }
    let mut out = Vec::with_capacity(input.len() / 2);
    let bytes = input.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let hi = (pair[0] as char).to_digit(16);
        let lo = (pair[1] as char).to_digit(16);
        match (hi, lo) {
            (Some(hi), Some(lo)) => out.push(((hi << 4) | lo) as u8),
            _ => {
                return Err(PdxError::bad_input("invalid hex digit")
                    .expected("0-9a-fA-F", String::from_utf8_lossy(pair).to_string()));
            }
        }
    }
    Ok(out)
}

/// Streams the file in 1 MiB blocks; never loads a large OBB into memory.
pub fn sha256_file(path: impl AsRef<Path>) -> Result<String> {
    let mut file = std::fs::File::open(path.as_ref())?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_encode(&hasher.finalize()))
}

pub fn verify_sha256(data: &[u8], expected_hex: &str) -> Result<()> {
    let actual = sha256_hex(data);
    if actual.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(PdxError::verification("SHA-256 mismatch").expected(expected_hex.to_string(), actual))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hex_round_trip() {
        let bytes = [0x00, 0x0f, 0x10, 0xff];
        assert_eq!(hex_decode(&hex_encode(&bytes)).unwrap(), bytes);
    }

    #[test]
    fn verify_reports_expected_and_actual() {
        let err = verify_sha256(b"abc", &"0".repeat(64)).unwrap_err();
        assert_eq!(err.kind, crate::error::ErrorKind::VerificationFailed);
        assert!(err.expected.is_some());
        assert!(err.actual.is_some());
    }
}
