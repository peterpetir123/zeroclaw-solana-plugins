//! Well-known Solana program IDs and constants.

/// SPL Token Program ID
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// Token-2022 (Token Extensions) Program ID
pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

/// System Program ID
pub const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";

/// SPL Associated Token Account Program ID
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

/// SPL Memo Program v2 ID
pub const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";

/// Rent sysvar
pub const SYSVAR_RENT_ID: &str = "SysvarRent111111111111111111111111111111111";

/// SPL Token Mint layout size (base, before Token-2022 extensions)
pub const MINT_LAYOUT_SIZE: usize = 82;

/// Account type byte offset for Token-2022 (after base Mint layout)
pub const TOKEN_2022_ACCOUNT_TYPE_OFFSET: usize = 165;

/// TLV extensions start offset for Token-2022 Mint accounts
pub const TOKEN_2022_EXTENSIONS_OFFSET: usize = 166;
