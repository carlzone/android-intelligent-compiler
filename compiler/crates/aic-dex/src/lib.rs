//! Deterministic writer for the intentionally narrow M0 DEX subset.

mod activity;
mod encoding;
mod integrity;
mod writer;

pub use activity::encode_activity_dex;
pub use encoding::{encode_mutf8, encode_sleb128, encode_uleb128, ByteWriter, DexError};
pub use writer::encode_minimal_dex;
