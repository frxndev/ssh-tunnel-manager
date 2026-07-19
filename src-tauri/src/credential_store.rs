use keyring::Entry;

const SERVICE: &str = "ssh-tunnel-manager";

/// Passwords and key passphrases are stored in the OS-native secure store
/// (Keychain on macOS, Credential Manager on Windows, Secret Service /
/// libsecret on Linux) rather than in the JSON profile file on disk.
pub fn store_secret(profile_id: &str, field: &str, value: &str) -> anyhow::Result<()> {
    let key = format!("{profile_id}:{field}");
    let entry = Entry::new(SERVICE, &key)?;
    entry.set_password(value)?;
    Ok(())
}

pub fn load_secret(profile_id: &str, field: &str) -> anyhow::Result<Option<String>> {
    let key = format!("{profile_id}:{field}");
    let entry = Entry::new(SERVICE, &key)?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn delete_secret(profile_id: &str, field: &str) -> anyhow::Result<()> {
    let key = format!("{profile_id}:{field}");
    let entry = Entry::new(SERVICE, &key)?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub fn delete_all_secrets(profile_id: &str) -> anyhow::Result<()> {
    for field in ["password", "passphrase"] {
        delete_secret(profile_id, field)?;
    }
    Ok(())
}
