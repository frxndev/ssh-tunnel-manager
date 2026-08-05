use tauri::{AppHandle, State};

use crate::models::{SshTemplate, TunnelProfile};
use crate::profile_store;
use crate::ssh_tunnel::TunnelManager;

#[tauri::command]
pub fn list_profiles(app: AppHandle) -> Result<Vec<TunnelProfile>, String> {
    profile_store::list_profiles(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_profile(app: AppHandle, profile: TunnelProfile) -> Result<TunnelProfile, String> {
    profile_store::save_profile(&app, profile).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_profile(app: AppHandle, id: String) -> Result<(), String> {
    profile_store::delete_profile(&app, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_tunnel(
    app: AppHandle,
    manager: State<TunnelManager>,
    id: String,
) -> Result<(), String> {
    let profile = profile_store::load_profile_with_secrets(&app, &id).map_err(|e| e.to_string())?;
    manager.start(app, profile).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn stop_tunnel(manager: State<TunnelManager>, id: String) -> Result<(), String> {
    manager.stop(&id);
    Ok(())
}

#[tauri::command]
pub fn list_templates(app: AppHandle) -> Result<Vec<SshTemplate>, String> {
    profile_store::list_templates(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_template(app: AppHandle, template: SshTemplate) -> Result<SshTemplate, String> {
    profile_store::save_template(&app, template).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_template(app: AppHandle, id: String) -> Result<(), String> {
    profile_store::delete_template(&app, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_profiles(app: AppHandle) -> Result<String, String> {
    profile_store::export_profiles(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_profiles(app: AppHandle, json: String) -> Result<u32, String> {
    profile_store::import_profiles(&app, &json).map_err(|e| e.to_string())
}

/// Writes the exported JSON straight to disk at `path` (chosen via the
/// native save dialog on the frontend), so the frontend doesn't need a
/// separate filesystem plugin just for this.
#[tauri::command]
pub fn export_profiles_to_file(app: AppHandle, path: String) -> Result<(), String> {
    let json = profile_store::export_profiles(&app).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_profiles_from_file(app: AppHandle, path: String) -> Result<u32, String> {
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    profile_store::import_profiles(&app, &json).map_err(|e| e.to_string())
}
