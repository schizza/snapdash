use keyring::Entry;

const SERVICE: &str = "dev.snapdash.Snapdash";
const USER: &str = "home-assistant-token";

pub fn set_ha_token(token: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE, USER).map_err(|e| e.to_string())?;
    entry.set_password(token).map_err(|e| e.to_string())
}

pub fn get_ha_token() -> Result<String, String> {
    let entry = Entry::new(SERVICE, USER).map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
}

pub fn delete_ha_token() -> Result<(), String> {
    let entry = Entry::new(SERVICE, USER).map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())
}
