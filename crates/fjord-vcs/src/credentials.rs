//! Tokens kept in the operating system's credential store (Windows Credential
//! Manager, macOS Keychain, Secret Service on Linux) — never in Fjord's database
//! or config files, and never logged.

use crate::{Result, VcsError};

const SERVICE: &str = "fjord";

/// Credential store account names.
pub const GITHUB: &str = "github";

fn entry(account: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, account).map_err(store_error)
}

fn store_error(e: keyring::Error) -> VcsError {
    VcsError::Credentials(e.to_string())
}

/// The stored secret, or None if there is none (or no credential store is available).
pub fn get(account: &str) -> Option<String> {
    entry(account)
        .ok()?
        .get_password()
        .ok()
        .filter(|s| !s.trim().is_empty())
}

pub fn set(account: &str, secret: &str) -> Result<()> {
    entry(account)?
        .set_password(secret.trim())
        .map_err(store_error)
}

/// Removes the stored secret; succeeds if there was none.
pub fn delete(account: &str) -> Result<()> {
    match entry(account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(store_error(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a running credential store, so it's opt-in: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn round_trips_through_the_os_store() {
        let account = "fjord-test-roundtrip";
        set(account, " secret-value \n").unwrap();
        assert_eq!(get(account).as_deref(), Some("secret-value"));
        delete(account).unwrap();
        assert_eq!(get(account), None);
        delete(account).unwrap();
    }
}
