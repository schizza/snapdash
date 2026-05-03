//! Persists the Home Assistant access token in the OS keychain rather
//! than on disk, so it survives across runs without ending up in
//! plaintext config files or shell history.

use keyring::Entry;

// Both keys are stable: changing them orphans every previously stored
// token and forces users to re-enter it. Bump only with a migration.
const SERVICE: &str = "dev.snapdash.Snapdash";
const USER: &str = "home-assistant-token";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenPresence {
    Unchecked,
    Checking,
    Present,
    Missing,
    AccessFailed(String),
}

pub fn presence() -> TokenPresence {
    match Entry::new(SERVICE, USER).and_then(|entry| entry.get_password().map(|_| ())) {
        Ok(()) => TokenPresence::Present,
        Err(keyring::Error::NoEntry) => TokenPresence::Missing,
        Err(e) => TokenPresence::AccessFailed(e.to_string()),
    }
}

pub fn set(token: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE, USER).map_err(|e| e.to_string())?;
    entry.set_password(token).map_err(|e| e.to_string())
}

pub fn get_raw() -> keyring::Result<String> {
    Entry::new(SERVICE, USER).and_then(|entry| entry.get_password())
}

pub fn delete_raw() -> keyring::Result<()> {
    Entry::new(SERVICE, USER).and_then(|entry| entry.delete_credential())
}
