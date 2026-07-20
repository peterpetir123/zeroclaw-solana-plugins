//! Token-2022 TLV extension parser.
//!
//! Token-2022 mints store extensions as a TLV (Type-Length-Value) array
//! starting at offset 166 (after base Mint layout 82 bytes + padding to 165
//! + 1 byte AccountType).
//!
//! Each TLV entry:
//!   - extension_type: u16 LE
//!   - extension_length: u16 LE
//!   - payload: [u8; extension_length]

use serde::{Deserialize, Serialize};

/// Known Token-2022 extension types we check for risk assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionType {
    TransferFeeConfig,
    DefaultAccountState,
    MintCloseAuthority,
    TransferHook,
    PermanentDelegate,
    NonTransferable,
    InterestBearingConfig,
    ConfidentialTransferMint,
    MetadataPointer,
    TokenMetadata,
    GroupPointer,
    GroupMemberPointer,
    /// An extension type we recognize the ID for but don't parse in detail.
    Unknown(u16),
}

impl ExtensionType {
    /// Map the raw u16 extension type ID to our enum.
    pub fn from_u16(val: u16) -> Self {
        match val {
            1 => ExtensionType::TransferFeeConfig,
            2 => ExtensionType::DefaultAccountState,
            3 => ExtensionType::MintCloseAuthority,
            7 => ExtensionType::TransferHook,
            12 => ExtensionType::PermanentDelegate,
            13 => ExtensionType::NonTransferable,
            14 => ExtensionType::InterestBearingConfig,
            9 => ExtensionType::ConfidentialTransferMint,
            18 => ExtensionType::MetadataPointer,
            19 => ExtensionType::TokenMetadata,
            21 => ExtensionType::GroupPointer,
            22 => ExtensionType::GroupMemberPointer,
            other => ExtensionType::Unknown(other),
        }
    }
}

/// A parsed TLV extension entry.
#[derive(Debug, Clone)]
pub struct Extension {
    pub ext_type: ExtensionType,
    /// Raw payload bytes for this extension.
    pub payload: Vec<u8>,
}

/// Default account state values (for DefaultAccountState extension).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Uninitialized,
    Initialized,
    Frozen,
}

/// Parse all TLV extensions from a Token-2022 mint account's raw data.
///
/// `data` is the FULL account data buffer (including the base Mint layout).
/// Extensions start at byte offset 166.
///
/// Fail-closed: any parse error returns `Err`, never silently skips.
pub fn parse_extensions(data: &[u8]) -> Result<Vec<Extension>, String> {
    use crate::constants::TOKEN_2022_EXTENSIONS_OFFSET;

    if data.len() <= TOKEN_2022_EXTENSIONS_OFFSET {
        // No extensions present (account too short or exactly at boundary).
        return Ok(Vec::new());
    }

    let mut extensions = Vec::new();
    let mut offset = TOKEN_2022_EXTENSIONS_OFFSET;

    while offset + 4 <= data.len() {
        // Read extension type (u16 LE)
        let ext_type_raw = u16::from_le_bytes(
            data[offset..offset + 2]
                .try_into()
                .map_err(|_| format!("failed to read extension type at offset {offset}"))?,
        );

        // Extension type 0 can signal padding / end of extensions
        if ext_type_raw == 0 {
            break;
        }

        // Read extension length (u16 LE)
        let ext_len = u16::from_le_bytes(
            data[offset + 2..offset + 4]
                .try_into()
                .map_err(|_| format!("failed to read extension length at offset {offset}"))?,
        ) as usize;

        let payload_start = offset + 4;
        let payload_end = payload_start + ext_len;

        if payload_end > data.len() {
            return Err(format!(
                "extension at offset {offset} declares length {ext_len} but data ends at {}; \
                 refusing to parse incomplete extension (fail-closed)",
                data.len()
            ));
        }

        extensions.push(Extension {
            ext_type: ExtensionType::from_u16(ext_type_raw),
            payload: data[payload_start..payload_end].to_vec(),
        });

        offset = payload_end;
    }

    Ok(extensions)
}

/// Check if a DefaultAccountState extension sets accounts to Frozen by default.
pub fn is_default_frozen(ext: &Extension) -> bool {
    if ext.ext_type != ExtensionType::DefaultAccountState {
        return false;
    }
    // Payload: 1 byte state (0 = Uninitialized, 1 = Initialized, 2 = Frozen)
    ext.payload.first().copied() == Some(2)
}

/// Check if a TransferFeeConfig extension is present and has a non-zero fee.
pub fn has_transfer_fee(ext: &Extension) -> Option<(u16, u64)> {
    if ext.ext_type != ExtensionType::TransferFeeConfig {
        return None;
    }
    // TransferFeeConfig layout:
    // transfer_fee_config_authority: Pubkey (32) + withheld_amount: u64 (8)
    // + older_transfer_fee: TransferFee (epoch: u64, max_fee: u64, rate_bps: u16 = 18)
    // + newer_transfer_fee: TransferFee (18)
    // Total minimum: 32 + 8 + 18 + 18 = 76 bytes
    if ext.payload.len() < 76 {
        return None;
    }
    // newer_transfer_fee starts at offset 58 (32+8+18)
    let newer_offset = 58;
    // rate_bps is at offset +16 within TransferFee (after epoch u64 + max_fee u64)
    let rate_bps = u16::from_le_bytes([
        ext.payload[newer_offset + 16],
        ext.payload[newer_offset + 17],
    ]);
    let max_fee = u64::from_le_bytes(
        ext.payload[newer_offset + 8..newer_offset + 16]
            .try_into()
            .unwrap_or([0; 8]),
    );
    if rate_bps > 0 || max_fee > 0 {
        Some((rate_bps, max_fee))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::TOKEN_2022_EXTENSIONS_OFFSET;

    fn make_extension_data(ext_type: u16, payload: &[u8]) -> Vec<u8> {
        // Pad to TOKEN_2022_EXTENSIONS_OFFSET, then add TLV entry
        let mut data = vec![0u8; TOKEN_2022_EXTENSIONS_OFFSET];
        data.extend_from_slice(&ext_type.to_le_bytes());
        data.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        data.extend_from_slice(payload);
        data
    }

    #[test]
    fn parses_empty_extensions() {
        let data = vec![0u8; TOKEN_2022_EXTENSIONS_OFFSET];
        let exts = parse_extensions(&data).unwrap();
        assert!(exts.is_empty());
    }

    #[test]
    fn parses_single_extension() {
        let data = make_extension_data(12, &[1, 2, 3, 4]); // PermanentDelegate
        let exts = parse_extensions(&data).unwrap();
        assert_eq!(exts.len(), 1);
        assert_eq!(exts[0].ext_type, ExtensionType::PermanentDelegate);
        assert_eq!(exts[0].payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn fails_on_truncated_payload() {
        let mut data = vec![0u8; TOKEN_2022_EXTENSIONS_OFFSET];
        data.extend_from_slice(&7u16.to_le_bytes()); // TransferHook
        data.extend_from_slice(&100u16.to_le_bytes()); // claims 100 bytes
        data.extend_from_slice(&[0u8; 10]); // only 10 bytes available
        assert!(parse_extensions(&data).is_err());
    }

    #[test]
    fn detects_default_frozen() {
        let ext = Extension {
            ext_type: ExtensionType::DefaultAccountState,
            payload: vec![2], // Frozen
        };
        assert!(is_default_frozen(&ext));
    }

    #[test]
    fn detects_default_initialized_not_frozen() {
        let ext = Extension {
            ext_type: ExtensionType::DefaultAccountState,
            payload: vec![1], // Initialized
        };
        assert!(!is_default_frozen(&ext));
    }
}
