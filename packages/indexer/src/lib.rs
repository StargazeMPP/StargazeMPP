pub mod config;
pub mod sink;
pub mod stream;

/// Re-export the decoder crate so existing import paths
/// (`stargaze_indexer::events::decode_program_log`, etc.) keep working
/// for downstream callers and documentation that pre-dates the split.
pub use stargaze_events as events;
pub use stargaze_events::{decode_logs, decode_program_log, DecodedEvent};
