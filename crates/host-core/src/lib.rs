//! The `host-core` crate: the host side of an event.

pub mod capability;
pub mod db;
pub mod event_folder;
pub mod listener;

/// Name of this crate, used by the skeleton test.
pub const CRATE_NAME: &str = "host-core";

#[cfg(test)]
mod tests {
    #[test]
    fn crate_name_matches() {
        assert_eq!(super::CRATE_NAME, "host-core");
    }
}
