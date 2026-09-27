//! Adapters for `facet-core`'s ports that are built only on portable crates, so one implementation serves
//! every platform. A composition root constructs them and hands them in; the core never names this crate.

pub mod dialogs;
pub mod http;
pub mod loopback;
pub mod secrets;
