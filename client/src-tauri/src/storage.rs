use crate::{crypto::hash_text, models::{ClipboardHistoryItem, DeviceIdentity}};
use age::{secrecy::ExposeSecret, x25519};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::rngs::OsRng;
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use std::{fs, path::{Path, PathBuf}};
use tauri::{AppHandle, Manager};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

const DB_FILE: &str = "clipcloud.sqlite3";
const CAPTURE_INTERVAL_MS: u64 = 1500;

pub fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
  let dir = app
    .path()
    .app_data_dir()
    .map_err(|error| format!("unable to determine app data directory: {error}"))?;
  fs::create_dir_all(&dir).map_err(|error| format!("unable to create app data directory: {error}"))?;
  Ok(dir.join(DB_FILE))
}

pub fn now_string() -> String {
  OffsetDateTime::now_utc()
    .format(&Rfc3339)
    .unwrap_or_else(|_| OffsetDateTime::now_utc().unix_timestamp().to_string())
}

pub fn open_db(path: &Path) -> Result<Connection, String> {
  let connection = Connection::open(path).map_err(|error| format!("unable to open sqlite database: {error}"))?;
  connection
    .execute_batch(
      "
      PRAGMA journal_mode = WAL;
      CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
      );
      CREATE TABLE IF NOT EXISTS clipboard_items (
        id TEXT PRIMARY KEY,
        content TEXT NOT NULL,
        content_type TEXT NOT NULL,
        sync_state TEXT NOT NULL,
        created_at TEXT NOT NULL,
        last_synced_at TEXT,
        origin TEXT NOT NULL,
        content_hash TEXT NOT NULL,
        remote_record_id TEXT
      );
      CREATE TABLE IF NOT EXISTS trusted_devices (
        id TEXT PRIMARY KEY,
        device_name TEXT NOT NULL,
        approved_at TEXT NOT NULL,
        revoked_at TEXT,
        is_current INTEGER NOT NULL DEFAULT 0
      );
      CREATE TABLE IF NOT EXISTS pending_activations (
        enrollment_id TEXT PRIMARY KEY,
        activation_secret TEXT NOT NULL,
        server_url TEXT NOT NULL,
        requested_at TEXT NOT NULL
      );
      ",
    )
    .map_err(|error| format!("unable to initialize sqlite schema: {error}"))?;
  Ok(connection)
}

pub fn get_setting(connection: &Connection, key: &str) -> Result<Option<String>, String> {
  connection
    .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| row.get::<_, String>(0))
    .optional()
    .map_err(|error| format!("unable to read local setting {key}: {error}"))
}

pub fn set_setting(connection: &Connection, key: &str, value: &str) -> Result<(), String> {
  connection
    .execute(
      "INSERT INTO settings (key, value) VALUES (?1, ?2)
       ON CONFLICT(key) DO UPDATE SET value = excluded.value",
      params![key, value],
    )
    .map_err(|error| format!("unable to persist local setting {key}: {error}"))?;
  Ok(())
}

fn default_device_name() -> String {
  std::env::var("COMPUTERNAME")
    .or_else(|_| std::env::var("HOSTNAME"))
    .unwrap_or_else(|_| "This device".to_string())
}

pub fn ensure_device_identity(connection: &Connection) -> Result<DeviceIdentity, String> {
  if let (Some(device_id), Some(device_name), Some(public_key), Some(private_key)) = (
    get_setting(connection, "device_id")?,
    get_setting(connection, "device_name")?,
    get_setting(connection, "device_public_key")?,
    get_setting(connection, "device_private_key")?,
  ) {
    return Ok(DeviceIdentity {
      device_id,
      device_name,
      public_key,
      private_key,
    });
  }

  let identity = x25519::Identity::generate();
  let device = DeviceIdentity {
    device_id: Uuid::new_v4().to_string(),
    device_name: default_device_name(),
    public_key: identity.to_public().to_string(),
    private_key: identity.to_string().expose_secret().to_owned(),
  };

  set_setting(connection, "device_id", &device.device_id)?;
  set_setting(connection, "device_name", &device.device_name)?;
  set_setting(connection, "device_public_key", &device.public_key)?;
  set_setting(connection, "device_private_key", &device.private_key)?;
  Ok(device)
}

pub fn ensure_vault_material(connection: &Connection) -> Result<(String, String), String> {
  if let (Some(vault_key), Some(recovery_secret)) = (
    get_setting(connection, "vault_key")?,
    get_setting(connection, "recovery_secret")?,
  ) {
    return Ok((vault_key, recovery_secret));
  }

  let mut vault_key = [0_u8; 32];
  OsRng.fill_bytes(&mut vault_key);
  let vault_key = BASE64.encode(vault_key);
  let recovery_secret = crate::crypto::random_secret();

  set_setting(connection, "vault_key", &vault_key)?;
  set_setting(connection, "recovery_secret", &recovery_secret)?;
  Ok((vault_key, recovery_secret))
}

pub fn insert_clipboard_item(connection: &Connection, content: &str, origin: &str) -> Result<(), String> {
  let trimmed = content.trim();
  if trimmed.is_empty() {
    return Ok(());
  }
  let content_hash = hash_text(trimmed);
  let latest_hash = connection
    .query_row(
      "SELECT content_hash FROM clipboard_items ORDER BY created_at DESC LIMIT 1",
      [],
      |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|error| format!("unable to inspect latest clipboard item: {error}"))?;

  if latest_hash.as_deref() == Some(content_hash.as_str()) {
    return Ok(());
  }

  connection
    .execute(
      "INSERT INTO clipboard_items (
        id, content, content_type, sync_state, created_at, origin, content_hash
      ) VALUES (?1, ?2, 'text', 'local_only', ?3, ?4, ?5)",
      params![Uuid::new_v4().to_string(), trimmed, now_string(), origin, content_hash],
    )
    .map_err(|error| format!("unable to insert clipboard item: {error}"))?;
  Ok(())
}

pub fn list_local_items(connection: &Connection) -> Result<Vec<ClipboardHistoryItem>, String> {
  let mut statement = connection
    .prepare(
      "SELECT id, content, content_type, sync_state, created_at, last_synced_at, origin
       FROM clipboard_items
       ORDER BY created_at DESC
       LIMIT 120",
    )
    .map_err(|error| format!("unable to query clipboard items: {error}"))?;
  let rows = statement
    .query_map([], |row| {
      Ok(ClipboardHistoryItem {
        id: row.get(0)?,
        content: row.get(1)?,
        content_type: row.get(2)?,
        sync_state: row.get(3)?,
        created_at: row.get(4)?,
        last_synced_at: row.get(5)?,
        origin: row.get(6)?,
      })
    })
    .map_err(|error| format!("unable to map clipboard items: {error}"))?;
  rows.collect::<Result<Vec<_>, _>>().map_err(|error| format!("unable to collect clipboard items: {error}"))
}

pub fn spawn_clipboard_poller(app: AppHandle) {
  if cfg!(mobile) {
    return;
  }
  tauri::async_runtime::spawn(async move {
    let path = match db_path(&app) {
      Ok(path) => path,
      Err(error) => {
        log::error!("{error}");
        return;
      }
    };
    loop {
      let text = tauri::async_runtime::spawn_blocking(|| {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        clipboard.get_text().ok()
      })
      .await
      .ok()
      .flatten();

      if let Some(text) = text {
        if let Err(error) = open_db(&path).and_then(|db| insert_clipboard_item(&db, &text, "local")) {
          log::warn!("{error}");
        }
      }

      tokio::time::sleep(std::time::Duration::from_millis(CAPTURE_INTERVAL_MS)).await;
    }
  });
}
