use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::credential_store;
use crate::models::{SshTemplate, TunnelProfile};

const STORE_FILE: &str = "profiles.json";
const KEY: &str = "profiles";
const TEMPLATES_KEY: &str = "templates";

/// Strips secrets out of a profile before it's written to the plain JSON
/// store on disk; the secrets themselves live in the OS keychain.
fn strip_secrets(mut profile: TunnelProfile) -> TunnelProfile {
    profile.password = None;
    profile.passphrase = None;
    profile
}

pub fn list_profiles(app: &AppHandle) -> anyhow::Result<Vec<TunnelProfile>> {
    let store = app.store(STORE_FILE)?;
    let profiles = store
        .get(KEY)
        .map(|v| serde_json::from_value::<Vec<TunnelProfile>>(v.clone()))
        .transpose()?
        .unwrap_or_default();
    Ok(profiles)
}

/// Saves (creating or updating) a profile. Any `password`/`passphrase`
/// present on the incoming profile is moved into the keychain and stripped
/// from what gets persisted to disk. Returns the saved, secret-free profile.
pub fn save_profile(app: &AppHandle, mut profile: TunnelProfile) -> anyhow::Result<TunnelProfile> {
    if profile.id.is_empty() {
        profile.id = uuid::Uuid::new_v4().to_string();
    }

    if let Some(password) = &profile.password {
        credential_store::store_secret(&profile.id, "password", password)?;
    }
    if let Some(passphrase) = &profile.passphrase {
        credential_store::store_secret(&profile.id, "passphrase", passphrase)?;
    }

    let mut profiles = list_profiles(app)?;
    let persisted = strip_secrets(profile.clone());
    match profiles.iter_mut().find(|p| p.id == persisted.id) {
        Some(existing) => *existing = persisted.clone(),
        None => profiles.push(persisted.clone()),
    }

    let store = app.store(STORE_FILE)?;
    store.set(KEY, serde_json::to_value(&profiles)?);
    store.save()?;

    Ok(persisted)
}

pub fn delete_profile(app: &AppHandle, id: &str) -> anyhow::Result<()> {
    let mut profiles = list_profiles(app)?;
    profiles.retain(|p| p.id != id);

    let store = app.store(STORE_FILE)?;
    store.set(KEY, serde_json::to_value(&profiles)?);
    store.save()?;

    credential_store::delete_all_secrets(id)?;
    Ok(())
}

/// Loads a single profile with its secrets re-attached from the keychain,
/// ready to hand to the tunnel engine.
pub fn load_profile_with_secrets(app: &AppHandle, id: &str) -> anyhow::Result<TunnelProfile> {
    let mut profile = list_profiles(app)?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| anyhow::anyhow!("perfil no encontrado"))?;

    profile.password = credential_store::load_secret(&profile.id, "password")?;
    profile.passphrase = credential_store::load_secret(&profile.id, "passphrase")?;
    Ok(profile)
}

// ---------------------------------------------------------------------
// SSH connection templates — no secrets involved, so these are much
// simpler than profiles: just plain JSON in the same store file.
// ---------------------------------------------------------------------

pub fn list_templates(app: &AppHandle) -> anyhow::Result<Vec<SshTemplate>> {
    let store = app.store(STORE_FILE)?;
    let templates = store
        .get(TEMPLATES_KEY)
        .map(|v| serde_json::from_value::<Vec<SshTemplate>>(v.clone()))
        .transpose()?
        .unwrap_or_default();
    Ok(templates)
}

pub fn save_template(app: &AppHandle, mut template: SshTemplate) -> anyhow::Result<SshTemplate> {
    if template.id.is_empty() {
        template.id = uuid::Uuid::new_v4().to_string();
    }

    let mut templates = list_templates(app)?;
    match templates.iter_mut().find(|t| t.id == template.id) {
        Some(existing) => *existing = template.clone(),
        None => templates.push(template.clone()),
    }

    let store = app.store(STORE_FILE)?;
    store.set(TEMPLATES_KEY, serde_json::to_value(&templates)?);
    store.save()?;
    Ok(template)
}

pub fn delete_template(app: &AppHandle, id: &str) -> anyhow::Result<()> {
    let mut templates = list_templates(app)?;
    templates.retain(|t| t.id != id);

    let store = app.store(STORE_FILE)?;
    store.set(TEMPLATES_KEY, serde_json::to_value(&templates)?);
    store.save()?;
    Ok(())
}

// ---------------------------------------------------------------------
// Export / import — deliberately profiles-only, and deliberately
// secret-free (profiles on disk already never carry password/passphrase,
// so exporting them is just re-serializing what's already there). This
// makes the exported file safe to hand to a teammate or commit to a
// private config repo without leaking credentials.
// ---------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize)]
struct ExportBundle {
    format: String,
    profiles: Vec<TunnelProfile>,
}

pub fn export_profiles(app: &AppHandle) -> anyhow::Result<String> {
    let bundle = ExportBundle {
        format: "ssh-tunnel-manager-profiles-v1".to_string(),
        profiles: list_profiles(app)?,
    };
    Ok(serde_json::to_string_pretty(&bundle)?)
}

/// Imports profiles from a previously exported JSON bundle. Every imported
/// profile gets a **new** id (so importing never overwrites an existing
/// profile, even if the file came from this same machine) and secrets are
/// never part of the bundle, so nothing needs to touch the keychain here —
/// the person re-enters passwords/passphrases for imported profiles as
/// needed. Returns how many profiles were imported.
pub fn import_profiles(app: &AppHandle, json: &str) -> anyhow::Result<u32> {
    let bundle: ExportBundle = serde_json::from_str(json)
        .map_err(|e| anyhow::anyhow!("archivo de importación inválido: {e}"))?;

    let mut profiles = list_profiles(app)?;
    let mut imported = 0u32;
    for mut profile in bundle.profiles {
        profile.id = uuid::Uuid::new_v4().to_string();
        profile.password = None;
        profile.passphrase = None;
        profiles.push(profile);
        imported += 1;
    }

    let store = app.store(STORE_FILE)?;
    store.set(KEY, serde_json::to_value(&profiles)?);
    store.save()?;
    Ok(imported)
}
