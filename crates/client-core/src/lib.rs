//! The `client-core` crate: the client side shared by desktop and phone.

pub mod nearby;
pub mod tls_pin;

/// Name of this crate, used by the skeleton test.
pub const CRATE_NAME: &str = "client-core";

#[cfg(test)]
mod tests {
    #[test]
    fn crate_name_matches() {
        assert_eq!(super::CRATE_NAME, "client-core");
    }
}
