//! Facet's platform-blind core.
//!
//! Every platform capability is a trait declared here and implemented in one of the platform
//! crates, which inject it. See `docs/architecture.md`.
//!
//! **A path is not a platform capability.** Where this machine keeps its files is the composition root's to
//! work out, and it hands the answer in; what a Facet database is, and what goes in it, is decided here so
//! that both platform crates get the same one.

pub mod category;
pub mod database;
pub mod debug_log;
pub mod face;
pub mod setting;

#[cfg(test)]
pub(crate) mod testing {
    use rusqlite::Connection;

    /// A fresh in-memory database with the app DDL applied.
    pub fn seeded() -> Connection {
        crate::database::open(std::path::Path::new(":memory:"), crate::database::APPDATA_DDL)
            .expect("the app DDL should apply to an in-memory database")
    }
}
