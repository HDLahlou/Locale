//! The Locale interface contract (digest §5–§7, §10–§11).
//!
//! Every type that crosses a process or device boundary between the host core,
//! the client core, the phone bridge and the account service lives here. Other
//! crates import these types instead of declaring their own.
//!
//! The crate is compiled into the phone app, so it stays small: no I/O (no
//! networking, file or database access), no async runtime and no clock reads.
//! The only cryptography is Ed25519 signing and verification in [`credential`];
//! everything else fixes text and byte formats.
//!
//! Modules:
//! - [`transport`]: port, protocol version, event ID, join token, certificate
//!   fingerprint, host code, join URL and `locale://join` deep link (§5.1).
//! - [`codes`]: the error body and the reason and error code groups (§5.2).
//! - [`api`]: host API routes, headers, request and response bodies, rate limits (§5.3).
//! - [`ws`]: index sync WebSocket messages and the change envelope (§5.4).
//! - [`service`]: account service API, credential format, proof of possession (§5.5).
//! - [`credential`]: credential signing and offline verification, the join
//!   proof of possession, and the host's verified-or-guest decision (§5.5, §8).
//! - [`mdns`]: mDNS service type and TXT record (§5.6).
//! - [`model`]: enums and records of the data model that cross the wire (§6).
//! - [`state`]: the item state machine as a table (§7).
//! - [`config`]: tunable configuration with bounds, and fixed constants (§10, §11).
//! - [`contract`]: renders the JSON Schema files and `CONTRACT.md` from the code.
//!
//! **Open and closed enums.** An *open* enum maps a value this version does not
//! know to its `Unknown` variant instead of failing, so a newer peer cannot break
//! an older one. A *closed* enum rejects unknown values. The split is listed in
//! `CONTRACT.md` and on each enum (`OPEN` constant).
#![deny(missing_docs)]

#[macro_use]
mod macros;

pub mod api;
pub mod codes;
pub mod config;
pub mod contract;
pub mod credential;
mod encoding;
mod error;
pub mod mdns;
pub mod model;
pub mod service;
pub mod state;
pub mod transport;
pub mod ws;

pub use error::ParseError;
pub use transport::{DEFAULT_PORT, PROTO_VERSION};

/// Name of this crate, used by the skeleton test.
pub const CRATE_NAME: &str = "proto";

#[cfg(test)]
mod tests {
    #[test]
    fn crate_name_matches() {
        assert_eq!(super::CRATE_NAME, "proto");
    }
}
