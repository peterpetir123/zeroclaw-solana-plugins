//! Minimal Solana primitives for wasm32-wasip2 plugins.
//!
//! This crate provides lightweight implementations of Solana types
//! (Pubkey, SPL Token Mint layout, Token-2022 TLV extensions, transaction
//! wire format) without depending on `solana-sdk` or `solana-client`,
//! which do not compile cleanly for `wasm32-wasip2`.
//!
//! All types are `no_std`-friendly where possible and designed for use
//! inside ZeroClaw tool plugins that communicate with Solana via JSON-RPC
//! over `wasi:http`.

pub mod pubkey;
pub mod rpc;
pub mod token2022;
pub mod wire;
pub mod mint;
pub mod constants;
