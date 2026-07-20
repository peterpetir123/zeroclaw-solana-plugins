//! SPL Token Mint layout parser (C-style Pod layout, not Borsh).
//!
//! Layout (82 bytes total):
//!   - mint_authority: COption<Pubkey> = 4 bytes tag + 32 bytes value = 36 bytes
//!   - supply: u64 = 8 bytes
//!   - decimals: u8 = 1 byte
//!   - is_initialized: bool = 1 byte
//!   - freeze_authority: COption<Pubkey> = 4 bytes tag + 32 bytes value = 36 bytes

use crate::pubkey::Pubkey;

/// Parsed SPL Token Mint base layout.
#[derive(Debug, Clone)]
pub struct MintLayout {
    pub mint_authority: Option<Pubkey>,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: Option<Pubkey>,
}

/// Parse the base SPL Token Mint layout from raw bytes.
///
/// Expects at least 82 bytes. For Token-2022 mints, pass the full buffer;
/// only the first 82 bytes are consumed here.
pub fn parse_mint_layout(data: &[u8]) -> Result<MintLayout, String> {
    if data.len() < 82 {
        return Err(format!(
            "mint data too short: expected >=82 bytes, got {}",
            data.len()
        ));
    }

    let mint_authority = parse_coption_pubkey(&data[0..36])?;
    let supply = u64::from_le_bytes(
        data[36..44]
            .try_into()
            .map_err(|_| "failed to read supply bytes")?,
    );
    let decimals = data[44];
    let is_initialized = data[45] != 0;
    let freeze_authority = parse_coption_pubkey(&data[46..82])?;

    if !is_initialized {
        return Err("mint account is not initialized".to_string());
    }

    Ok(MintLayout {
        mint_authority,
        supply,
        decimals,
        is_initialized,
        freeze_authority,
    })
}

/// Parse a COption<Pubkey> from 36 bytes:
/// - bytes[0..4]: u32 LE tag (0 = None, 1 = Some)
/// - bytes[4..36]: Pubkey (only meaningful when tag == 1)
fn parse_coption_pubkey(data: &[u8]) -> Result<Option<Pubkey>, String> {
    if data.len() < 36 {
        return Err("COption<Pubkey> data too short".to_string());
    }
    let tag = u32::from_le_bytes(
        data[0..4]
            .try_into()
            .map_err(|_| "failed to read COption tag")?,
    );
    match tag {
        0 => Ok(None),
        1 => {
            let mut key_bytes = [0u8; 32];
            key_bytes.copy_from_slice(&data[4..36]);
            Ok(Some(Pubkey::from_bytes(key_bytes)))
        }
        _ => Err(format!("invalid COption tag: {tag}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mint_data(
        mint_auth: Option<[u8; 32]>,
        supply: u64,
        decimals: u8,
        freeze_auth: Option<[u8; 32]>,
    ) -> Vec<u8> {
        let mut data = Vec::with_capacity(82);
        // mint_authority COption<Pubkey>
        match mint_auth {
            Some(key) => {
                data.extend_from_slice(&1u32.to_le_bytes());
                data.extend_from_slice(&key);
            }
            None => {
                data.extend_from_slice(&0u32.to_le_bytes());
                data.extend_from_slice(&[0u8; 32]);
            }
        }
        // supply
        data.extend_from_slice(&supply.to_le_bytes());
        // decimals
        data.push(decimals);
        // is_initialized
        data.push(1);
        // freeze_authority COption<Pubkey>
        match freeze_auth {
            Some(key) => {
                data.extend_from_slice(&1u32.to_le_bytes());
                data.extend_from_slice(&key);
            }
            None => {
                data.extend_from_slice(&0u32.to_le_bytes());
                data.extend_from_slice(&[0u8; 32]);
            }
        }
        data
    }

    #[test]
    fn parse_clean_mint() {
        let data = make_mint_data(None, 1_000_000_000, 9, None);
        let m = parse_mint_layout(&data).unwrap();
        assert!(m.mint_authority.is_none());
        assert!(m.freeze_authority.is_none());
        assert_eq!(m.supply, 1_000_000_000);
        assert_eq!(m.decimals, 9);
    }

    #[test]
    fn parse_mint_with_authorities() {
        let key = [42u8; 32];
        let data = make_mint_data(Some(key), 500, 6, Some(key));
        let m = parse_mint_layout(&data).unwrap();
        assert!(m.mint_authority.is_some());
        assert!(m.freeze_authority.is_some());
        assert_eq!(m.mint_authority.unwrap().0, key);
    }

    #[test]
    fn rejects_too_short() {
        let data = vec![0u8; 50];
        assert!(parse_mint_layout(&data).is_err());
    }

    #[test]
    fn rejects_uninitialized() {
        let mut data = make_mint_data(None, 0, 0, None);
        data[45] = 0; // is_initialized = false
        assert!(parse_mint_layout(&data).is_err());
    }
}
