//! Deterministic writer for the intentionally narrow M0 DEX subset.

mod activity;
mod encoding;
mod integrity;
mod lir;
mod lower;
mod writer;

pub use activity::encode_activity_dex;
pub use encoding::{encode_mutf8, encode_sleb128, encode_uleb128, ByteWriter, DexError};
pub use lir::{
    assemble as assemble_lir, Instruction as LirInstruction, Label, Register, ValueKind,
};
pub use lower::{
    lower_events, lower_function, lower_function_typed, lower_on_click, lower_on_create,
    FunctionTarget, LoweredEvents, LoweredMethod, StringLowering, UiLowering,
};
pub use writer::encode_minimal_dex;
