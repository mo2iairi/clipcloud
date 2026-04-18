mod commands;
mod crypto;
mod models;
mod storage;

use commands::{
  activate_enrollment, approve_enrollment, bootstrap_server, fetch_synced_history,
  generate_enrollment_code, get_app_snapshot, join_with_enrollment_code, list_pending_enrollments,
  list_trusted_devices, recover_access, revoke_trusted_device, sync_history_item,
};
use storage::{db_path, open_db, spawn_clipboard_poller};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      let handle = app.handle().clone();
      let _ = open_db(&db_path(&handle)?);
      spawn_clipboard_poller(handle);

      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      get_app_snapshot,
      bootstrap_server,
      generate_enrollment_code,
      join_with_enrollment_code,
      activate_enrollment,
      list_trusted_devices,
      list_pending_enrollments,
      approve_enrollment,
      revoke_trusted_device,
      sync_history_item,
      fetch_synced_history,
      recover_access
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
