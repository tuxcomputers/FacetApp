//! Facet's platform-blind core.
//!
//! Every platform capability is a trait declared here and implemented in one of the platform
//! crates, which inject it. See `docs/architecture.md`.
//!
//! **A path is not a platform capability.** Where this machine keeps its files is the composition root's to
//! work out, and it hands the answer in; what a Facet database is, and what goes in it, is decided here so
//! that both platform crates get the same one.

pub mod database;
pub mod debug_log;
pub mod setting;
