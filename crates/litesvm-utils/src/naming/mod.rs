//! Naming vocabulary: human names for pubkeys, errors, instructions, and
//! events, so failure output and logs read in the test's own vocabulary.

pub mod actors;
pub mod aliases;
pub mod error_names;
pub mod events;
pub mod instruction_names;

pub use actors::{deterministic_keypair, seed_bytes, ActorRegistry};
pub use aliases::Aliases;
pub use error_names::ErrorNames;
pub use events::{EventInfo, EventRegistry};
pub use instruction_names::InstructionNames;
