//! Facet's platform-blind core.
//!
//! Every platform capability is a trait declared here and implemented in one of the platform
//! crates, which inject it. See `docs/architecture.md`.

/// The DDL, in the order it is applied. Compiled in, so a shipped binary has nothing to find at
/// runtime; `docs/port-findings.md` records why that matters.
pub const DDL: &[(&str, &str)] = &[];
