//! Persists the Home Assistant access token in the OS keychain rather
//! than on disk, so it survives across runs without ending up in
//! plaintext config files or shell history.

use keyring::Entry;

// Both keys are stable: changing them orphans every previously stored
// token and forces users to re-enter it. Bump only with a migration.
const SERVICE: &str = "dev.snapdash.Snapdash";
const USER: &str = "home-assistant-token";

pub fn set(token: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE, USER).map_err(|e| e.to_string())?;
    entry.set_password(token).map_err(|e| e.to_string())
}

pub fn get() -> Result<String, String> {
    let entry = Entry::new(SERVICE, USER).map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
}

pub fn delete() -> Result<(), String> {
    let entry = Entry::new(SERVICE, USER).map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())
}
