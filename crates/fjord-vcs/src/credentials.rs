//! Tokens kept in the operating system's credential store (Windows Credential
//! Manager, macOS Keychain, Secret Service on Linux) — never in Fjord's database
//! or config files, and never logged.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::{Result, VcsError};

const SERVICE: &str = "fjord";

/// Reading the OS store can take a while (a D-Bus round trip on Linux) and happens for
/// every request, so reads are remembered briefly. Saving or deleting clears the entry.
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);

/// Account → when it was read and what was there.
type Reads = HashMap<String, (Instant, Option<String>)>;

static CACHE: Mutex<Option<Reads>> = Mutex::new(None);

fn forget(account: &str) {
    if let Some(map) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        map.remove(account);
    }
}

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
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let map = cache.get_or_insert_with(HashMap::new);
    if let Some((at, secret)) = map.get(account)
        && at.elapsed() < CACHE_TTL
    {
        return secret.clone();
    }
    let secret = read(account);
    map.insert(account.to_string(), (Instant::now(), secret.clone()));
    secret
}

fn read(account: &str) -> Option<String> {
    entry(account)
        .ok()?
        .get_password()
        .ok()
        .filter(|s| !s.trim().is_empty())
}

pub fn set(account: &str, secret: &str) -> Result<()> {
    forget(account);
    entry(account)?
        .set_password(secret.trim())
        .map_err(store_error)
}

/// Removes the stored secret; succeeds if there was none.
pub fn delete(account: &str) -> Result<()> {
    forget(account);
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
