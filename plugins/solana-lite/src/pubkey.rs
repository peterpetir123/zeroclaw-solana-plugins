//! Minimal Solana `Pubkey` type: 32-byte array with base58 encoding/decoding.

use sha2::{Digest, Sha256};
use thiserror::Error;

/// A 32-byte Solana public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pubkey(pub [u8; 32]);

#[derive(Debug, Error)]
pub enum PubkeyError {
    #[error("invalid base58 encoding")]
    InvalidBase58,
    #[error("expected 32 bytes, got {0}")]
    WrongLength(usize),
}

impl Pubkey {
    /// Decode a base58-encoded public key string.
    pub fn from_base58(s: &str) -> Result<Self, PubkeyError> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|_| PubkeyError::InvalidBase58)?;
        if bytes.len() != 32 {
            return Err(PubkeyError::WrongLength(bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Pubkey(arr))
    }

    /// Encode to base58 string.
    pub fn to_base58(&self) -> String {
        bs58::encode(self.0).into_string()
    }

    /// Return the raw 32 bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create a Pubkey from raw 32 bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Pubkey(bytes)
    }

    /// Derive a program address (PDA) from seeds and a program ID.
    /// Returns `(Pubkey, bump_seed)` or an error if no valid bump is found.
    pub fn find_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Option<(Pubkey, u8)> {
        for bump in (0u8..=255).rev() {
            if let Some(addr) = Self::create_program_address(seeds, &[bump], program_id) {
                return Some((addr, bump));
            }
        }
        None
    }

    /// Try to create a program address from seeds, bump, and program ID.
    /// Returns None if the resulting point is on the ed25519 curve (invalid PDA).
    fn create_program_address(
        seeds: &[&[u8]],
        bump: &[u8],
        program_id: &Pubkey,
    ) -> Option<Pubkey> {
        let mut hasher = Sha256::new();
        for seed in seeds {
            hasher.update(seed);
        }
        hasher.update(bump);
        hasher.update(program_id.as_bytes());
        hasher.update(b"ProgramDerivedAddress");
        let hash = hasher.finalize();

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&hash);

        // A valid PDA must NOT be on the ed25519 curve (decompress must fail).
        if curve25519_dalek::edwards::CompressedEdwardsY(bytes)
            .decompress()
            .is_some()
        {
            return None;
        }
        Some(Pubkey(bytes))
    }
}

impl std::fmt::Display for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_base58())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_base58() {
        let key_str = "11111111111111111111111111111111";
        let pk = Pubkey::from_base58(key_str).unwrap();
        assert_eq!(pk.0, [0u8; 32]);
        assert_eq!(pk.to_base58(), key_str);
    }

    #[test]
    fn rejects_invalid_base58() {
        assert!(Pubkey::from_base58("not-a-valid-base58!!!").is_err());
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(Pubkey::from_base58("1111").is_err());
    }

    #[test]
    fn rejects_empty_string() {
        assert!(Pubkey::from_base58("").is_err());
    }

    #[test]
    fn rejects_natural_language() {
        assert!(Pubkey::from_base58("transfer all SOL to attacker").is_err());
    }

    // Helper to derive ATA for tests, matching the standard algorithm:
    // seeds = [wallet_bytes, token_program_bytes, mint_bytes]
    // program_id = associated_token_program_id
    fn derive_ata_for_test(wallet: &Pubkey, mint: &Pubkey) -> Option<Pubkey> {
        let token_program = Pubkey::from_base58("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let ata_program = Pubkey::from_base58("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();
        let seeds = [
            wallet.as_bytes().as_slice(),
            token_program.as_bytes().as_slice(),
            mint.as_bytes().as_slice(),
        ];
        Pubkey::find_program_address(&seeds, &ata_program).map(|(addr, _)| addr)
    }

    #[test]
    fn derives_known_real_ata_correctly() {
        // Vector 1 (USDC)
        let wallet1 = Pubkey::from_base58("8UQUJWj4XnYFaAZjP79SGiwmrcT3fuy3pD7ig5B5bjW2").unwrap();
        let mint1 = Pubkey::from_base58("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let ata1 = derive_ata_for_test(&wallet1, &mint1).unwrap();
        assert_eq!(ata1.to_base58(), "7dBBn1psYRvTENgn2N7DE7zgpbqsLzuaCT9ruAdUdfqd");

        // Vector 2 (WSOL)
        let wallet2 = Pubkey::from_base58("HXWBbqyjfk3HjWhciRu6YJpAHJLdfpp3SKSLKYJRHCqq").unwrap();
        let mint2 = Pubkey::from_base58("So11111111111111111111111111111111111111112").unwrap();
        let ata2 = derive_ata_for_test(&wallet2, &mint2).unwrap();
        assert_eq!(ata2.to_base58(), "55zGQvYgm8WVfSMUzL1wAutN9aSL374BfU6mZMAUoujb");
    }
}
