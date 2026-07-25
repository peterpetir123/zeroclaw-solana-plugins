//! Solana transaction wire format: compact-u16, Message v0 serialization,
//! and unsigned transaction wrapper.
//!
//! These functions produce raw bytes matching the Solana wire format
//! without depending on `solana-sdk`.

use crate::pubkey::Pubkey;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// A minimal Solana instruction (program_id index, account metas, data).
#[derive(Debug, Clone)]
pub struct Instruction {
    /// The program ID (as Pubkey).
    pub program_id: Pubkey,
    /// Account metas: (pubkey, is_signer, is_writable).
    pub accounts: Vec<(Pubkey, bool, bool)>,
    /// Instruction data.
    pub data: Vec<u8>,
}

/// Encode a u16 as Solana's compact-u16 (short-vec) format.
pub fn write_compact_u16(buf: &mut Vec<u8>, mut n: u16) {
    loop {
        let mut byte = (n & 0x7f) as u8;
        n >>= 7;
        if n != 0 {
            byte |= 0x80;
            buf.push(byte);
        } else {
            buf.push(byte);
            break;
        }
    }
}

/// Serialize a Solana Message v0 (versioned) from a fee payer, instructions,
/// and a recent blockhash.
///
/// Returns the raw message bytes. This implements the Versioned Message v0 format.
pub fn serialize_v0_message(
    fee_payer: &Pubkey,
    instructions: &[Instruction],
    blockhash: &str,
) -> Result<Vec<u8>, String> {
    // Collect all unique accounts in the required order:
    // 1. Fee payer (always first, signer + writable)
    // 2. Other signers (writable first, then read-only)
    // 3. Non-signers (writable first, then read-only)
    let mut accounts_map: Vec<(Pubkey, bool, bool)> = Vec::new(); // (pubkey, is_signer, is_writable)

    // Add fee payer first
    accounts_map.push((*fee_payer, true, true));

    // Add all accounts from instructions
    for ix in instructions {
        for (pk, is_signer, is_writable) in &ix.accounts {
            if let Some(existing) = accounts_map.iter_mut().find(|(p, _, _)| p == pk) {
                // Merge: promote to signer/writable if needed
                existing.1 = existing.1 || *is_signer;
                existing.2 = existing.2 || *is_writable;
            } else {
                accounts_map.push((*pk, *is_signer, *is_writable));
            }
        }
        // Add program IDs as non-signer, non-writable
        if !accounts_map.iter().any(|(p, _, _)| p == &ix.program_id) {
            accounts_map.push((ix.program_id, false, false));
        }
    }

    // Sort accounts: signers first (writable before read-only),
    // then non-signers (writable before read-only).
    // Fee payer stays at index 0.
    let fee_payer_entry = accounts_map.remove(0);
    accounts_map.sort_by(|a, b| {
        let a_order = (!a.1 as u8, !a.2 as u8);
        let b_order = (!b.1 as u8, !b.2 as u8);
        a_order.cmp(&b_order)
    });
    accounts_map.insert(0, fee_payer_entry);

    if accounts_map.len() > 255 {
        return Err(format!(
            "too many accounts for v0 message: {} (max 255)",
            accounts_map.len()
        ));
    }

    let num_required_signatures_count = accounts_map.iter().filter(|(_, s, _)| *s).count();
    if num_required_signatures_count > 255 {
        return Err(format!(
            "too many signers: {} (max 255)",
            num_required_signatures_count
        ));
    }
    let num_required_signatures = num_required_signatures_count as u8;

    let num_readonly_signed_count = accounts_map
        .iter()
        .filter(|(_, s, w)| *s && !*w)
        .count();
    if num_readonly_signed_count > 255 {
        return Err(format!(
            "too many readonly signers: {} (max 255)",
            num_readonly_signed_count
        ));
    }
    let num_readonly_signed = num_readonly_signed_count as u8;

    let num_readonly_unsigned_count = accounts_map
        .iter()
        .filter(|(_, s, w)| !*s && !*w)
        .count();
    if num_readonly_unsigned_count > 255 {
        return Err(format!(
            "too many readonly unsigned accounts: {} (max 255)",
            num_readonly_unsigned_count
        ));
    }
    let num_readonly_unsigned = num_readonly_unsigned_count as u8;

    let blockhash_bytes = bs58::decode(blockhash)
        .into_vec()
        .map_err(|_| "invalid blockhash base58")?;
    if blockhash_bytes.len() != 32 {
        return Err(format!(
            "blockhash must be 32 bytes, got {}",
            blockhash_bytes.len()
        ));
    }

    // Serialize message v0
    let mut msg = Vec::new();

    // Version prefix: 0x80 (128) indicates Versioned Message v0
    msg.push(0x80);

    // Header
    msg.push(num_required_signatures);
    msg.push(num_readonly_signed);
    msg.push(num_readonly_unsigned);

    // Account addresses
    write_compact_u16(&mut msg, accounts_map.len() as u16);
    for (pk, _, _) in &accounts_map {
        msg.extend_from_slice(pk.as_bytes());
    }

    // Recent blockhash
    msg.extend_from_slice(&blockhash_bytes);

    // Instructions
    write_compact_u16(&mut msg, instructions.len() as u16);
    for ix in instructions {
        // Program ID index
        let prog_idx = accounts_map
            .iter()
            .position(|(p, _, _)| p == &ix.program_id)
            .ok_or("program ID not found in accounts list")?;
        if prog_idx > 255 {
            return Err(format!("program ID index overflow: {prog_idx} (max 255)"));
        }
        msg.push(prog_idx as u8);

        // Account indices
        write_compact_u16(&mut msg, ix.accounts.len() as u16);
        for (pk, _, _) in &ix.accounts {
            let idx = accounts_map
                .iter()
                .position(|(p, _, _)| p == pk)
                .ok_or("account not found in accounts list")?;
            if idx > 255 {
                return Err(format!("account index overflow: {idx} (max 255)"));
            }
            msg.push(idx as u8);
        }

        // Data
        write_compact_u16(&mut msg, ix.data.len() as u16);
        msg.extend_from_slice(&ix.data);
    }

    // Address table lookups: 0 (compact-u16)
    write_compact_u16(&mut msg, 0);

    Ok(msg)
}

/// Wrap a serialized message into an unsigned transaction.
///
/// Prepends `num_signers` empty (all-zero) 64-byte signature placeholders,
/// then the message bytes. Returns the full transaction bytes.
pub fn wrap_unsigned_transaction(message_bytes: &[u8], num_signers: u8) -> Vec<u8> {
    let mut tx = Vec::new();
    write_compact_u16(&mut tx, num_signers as u16);
    for _ in 0..num_signers {
        tx.extend_from_slice(&[0u8; 64]); // empty signature placeholder
    }
    tx.extend_from_slice(message_bytes);
    tx
}

/// Encode bytes to base64 string.
pub fn base64_encode(data: &[u8]) -> String {
    BASE64.encode(data)
}

/// Decode base64 string to bytes.
pub fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    BASE64.decode(s).map_err(|e| format!("base64 decode error: {e}"))
}

/// Build a System Program transfer instruction (SOL transfer).
pub fn build_system_transfer_ix(from: &Pubkey, to: &Pubkey, lamports: u64) -> Instruction {
    let system_program = Pubkey::from_base58(crate::constants::SYSTEM_PROGRAM_ID).unwrap();
    // System instruction index 2 = Transfer
    let mut data = Vec::with_capacity(12);
    data.extend_from_slice(&2u32.to_le_bytes()); // instruction index
    data.extend_from_slice(&lamports.to_le_bytes());

    Instruction {
        program_id: system_program,
        accounts: vec![
            (*from, true, true),  // from (signer, writable)
            (*to, false, true),   // to (writable)
        ],
        data,
    }
}

/// Build a Memo Program instruction.
pub fn build_memo_ix(memo_text: &str) -> Instruction {
    let memo_program = Pubkey::from_base58(crate::constants::MEMO_PROGRAM_ID).unwrap();
    Instruction {
        program_id: memo_program,
        accounts: vec![],
        data: memo_text.as_bytes().to_vec(),
    }
}

/// Derive the Associated Token Account (ATA) address for a given wallet and mint.
pub fn derive_ata(wallet: &Pubkey, mint: &Pubkey) -> Result<Pubkey, String> {
    let ata_program = Pubkey::from_base58(crate::constants::ASSOCIATED_TOKEN_PROGRAM_ID)
        .map_err(|e| format!("invalid ATA program ID: {e}"))?;
    let token_program = Pubkey::from_base58(crate::constants::TOKEN_PROGRAM_ID)
        .map_err(|e| format!("invalid token program ID: {e}"))?;

    Pubkey::find_program_address(
        &[
            wallet.as_bytes(),
            token_program.as_bytes(),
            mint.as_bytes(),
        ],
        &ata_program,
    )
    .map(|(addr, _)| addr)
    .ok_or_else(|| "failed to derive ATA address".to_string())
}

/// Build an instruction to create an Associated Token Account.
pub fn build_create_ata_ix(
    funder: &Pubkey,
    wallet: &Pubkey,
    mint: &Pubkey,
    ata: &Pubkey,
) -> Instruction {
    let ata_program = Pubkey::from_base58(crate::constants::ASSOCIATED_TOKEN_PROGRAM_ID).unwrap();
    let token_program = Pubkey::from_base58(crate::constants::TOKEN_PROGRAM_ID).unwrap();
    let system_program = Pubkey::from_base58(crate::constants::SYSTEM_PROGRAM_ID).unwrap();

    Instruction {
        program_id: ata_program,
        accounts: vec![
            (*funder, true, true),        // funder (signer, writable)
            (*ata, false, true),           // ATA to create (writable)
            (*wallet, false, false),       // wallet owner
            (*mint, false, false),         // mint
            (system_program, false, false), // System Program
            (token_program, false, false),  // Token Program
        ],
        data: vec![1], // CreateIdempotent instruction (index 1) for on-chain race condition defense
    }
}

/// Build an SPL Token Transfer instruction.
pub fn build_spl_transfer_ix(
    source_ata: &Pubkey,
    dest_ata: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> Instruction {
    let token_program = Pubkey::from_base58(crate::constants::TOKEN_PROGRAM_ID).unwrap();
    // SPL Token instruction index 3 = Transfer
    let mut data = Vec::with_capacity(9);
    data.push(3); // instruction index
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: token_program,
        accounts: vec![
            (*source_ata, false, true),  // source (writable)
            (*dest_ata, false, true),    // destination (writable)
            (*owner, true, false),       // owner (signer)
        ],
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_u16_small() {
        let mut buf = Vec::new();
        write_compact_u16(&mut buf, 5);
        assert_eq!(buf, vec![5]);
    }

    #[test]
    fn compact_u16_128() {
        let mut buf = Vec::new();
        write_compact_u16(&mut buf, 128);
        assert_eq!(buf, vec![0x80, 0x01]);
    }

    #[test]
    fn compact_u16_zero() {
        let mut buf = Vec::new();
        write_compact_u16(&mut buf, 0);
        assert_eq!(buf, vec![0]);
    }

    #[test]
    fn system_transfer_ix_structure() {
        let from = Pubkey::from_bytes([1u8; 32]);
        let to = Pubkey::from_bytes([2u8; 32]);
        let ix = build_system_transfer_ix(&from, &to, 1_000_000);
        assert_eq!(ix.accounts.len(), 2);
        assert_eq!(ix.data.len(), 12); // 4 bytes index + 8 bytes lamports
    }

    #[test]
    fn memo_ix_contains_text() {
        let ix = build_memo_ix("invoice #42");
        assert_eq!(ix.data, b"invoice #42");
        assert!(ix.accounts.is_empty());
    }

    #[test]
    fn unsigned_tx_has_empty_signatures() {
        let msg = vec![1, 2, 3, 4, 5];
        let tx = wrap_unsigned_transaction(&msg, 1);
        // compact_u16(1) = [1], then 64 zero bytes, then message
        assert_eq!(tx[0], 1);
        assert_eq!(&tx[1..65], &[0u8; 64]);
        assert_eq!(&tx[65..], &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn base64_roundtrip() {
        let data = b"hello solana";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn serialize_v0_message_structure() {
        let payer = Pubkey::from_bytes([1u8; 32]);
        let to = Pubkey::from_bytes([2u8; 32]);
        let ix = build_system_transfer_ix(&payer, &to, 1000);
        let blockhash = "GHtXQBpokMJhbUyHQDiKvJvPchsb4xRuvfFwkdSEiMPQ";
        
        let msg = serialize_v0_message(&payer, &[ix], blockhash).unwrap();
        
        // Byte 0 must be 0x80 (prefix for v0 message format)
        assert_eq!(msg[0], 0x80);
        
        // Header starts at byte 1:
        // num_required_signatures = 1
        assert_eq!(msg[1], 1);
        // num_readonly_signed = 0
        assert_eq!(msg[2], 0);
        // num_readonly_unsigned = 1 (System Program ID is readonly and unsigned)
        assert_eq!(msg[3], 1);
    }

    #[test]
    fn overflow_accounts_fails_closed() {
        let payer = Pubkey::from_bytes([1u8; 32]);
        let mut accounts = Vec::new();
        // Add 260 unique accounts to force accounts_map > 255
        for i in 0..260u16 {
            let mut b = [0u8; 32];
            let bytes = i.to_le_bytes();
            b[0] = bytes[0];
            b[1] = bytes[1];
            accounts.push((Pubkey::from_bytes(b), false, true));
        }
        let ix = Instruction {
            program_id: Pubkey::from_bytes([99u8; 32]),
            accounts,
            data: vec![],
        };
        let blockhash = "GHtXQBpokMJhbUyHQDiKvJvPchsb4xRuvfFwkdSEiMPQ";
        let res = serialize_v0_message(&payer, &[ix], blockhash);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("too many accounts"));
    }

    #[test]
    fn create_ata_idempotent_structure() {
        let funder = Pubkey::from_bytes([1u8; 32]);
        let wallet = Pubkey::from_bytes([2u8; 32]);
        let mint = Pubkey::from_bytes([3u8; 32]);
        let ata = Pubkey::from_bytes([4u8; 32]);

        let ix = build_create_ata_ix(&funder, &wallet, &mint, &ata);
        assert_eq!(ix.data, vec![1]); // 1 byte CreateIdempotent instruction
        assert_eq!(ix.accounts.len(), 6);
    }
}
