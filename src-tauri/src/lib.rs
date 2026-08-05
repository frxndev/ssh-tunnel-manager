mod commands;
mod credential_store;
mod diagnostics;
mod models;
mod profile_store;
mod ssh_tunnel;

use ssh_tunnel::TunnelManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(TunnelManager::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::start_tunnel,
            commands::stop_tunnel,
            commands::list_templates,
            commands::save_template,
            commands::delete_template,
            commands::export_profiles,
            commands::import_profiles,
            commands::export_profiles_to_file,
            commands::import_profiles_from_file,
            diagnostics::check_port_available,
            diagnostics::test_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error running the SSH tunnel manager app");
}
