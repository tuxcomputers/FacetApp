//! The TimeFlip2 cube: what it is called on the air, which device is one, how a login is judged, what its
//! commands look like, and the rows the app keeps about it. Nothing here touches a radio; the session code
//! drives a [`crate::port::Link`] it is handed.

pub mod command;
pub mod info;
pub mod login;
pub mod rows;
pub mod scan;
pub mod session;
pub mod uuids;

#[cfg(test)]
pub(crate) mod fake;
