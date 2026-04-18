use crate::{
  crypto::{
    decrypt_text, decrypt_with_recovery_secret, encrypt_text, encrypt_with_recovery_secret,
    hash_text, unwrap_vault_key_for_device, wrap_vault_key_for_device,
  },
  models::{
    ActivationResponse, AppSnapshot, BootstrapServerRequest, DeviceProfile, EnrollmentCode,
    JoinEnrollmentRequest, JoinEnrollmentResponse, PendingEnrollment, ServerBootstrapResponse,
    ServerClipboardItem, ServerEnrollmentCodeResponse, ServerJoinEnrollmentResponse,
    ServerPendingEnrollment, ServerTrustedDevice, TrustedDevice,
  },
  storage::{
    db_path, ensure_device_identity, ensure_vault_material, get_setting, list_local_items,
    now_string, open_db, set_setting,
  },
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use rusqlite::{params, OptionalExtension};
use tauri::AppHandle;
use uuid::Uuid;

async fn authorized_client_request(app: &AppHandle) -> Result<(reqwest::Client, String, String), String> {
  let db = open_db(&db_path(app)?)?;
  let server_url = get_setting(&db, "server_url")?.ok_or_else(|| "server has not been configured".to_string())?;
  let auth_token = get_setting(&db, "auth_token")?.ok_or_else(|| "device is not authenticated with the server".to_string())?;
  Ok((reqwest::Client::new(), server_url, auth_token))
}

#[tauri::command]
pub async fn get_app_snapshot(app: AppHandle) -> Result<AppSnapshot, String> {
  let db = open_db(&db_path(&app)?)?;
  let identity = ensure_device_identity(&db)?;
  Ok(AppSnapshot {
    device: DeviceProfile {
      device_id: identity.device_id,
      device_name: identity.device_name,
      capture_mode: if cfg!(mobile) { "manual".into() } else { "desktop-polling".into() },
      server_url: get_setting(&db, "server_url")?,
      is_connected_to_server: get_setting(&db, "auth_token")?.is_some(),
      vault_id: get_setting(&db, "vault_id")?,
    },
    items: list_local_items(&db)?,
  })
}

#[tauri::command]
pub async fn bootstrap_server(app: AppHandle, request: BootstrapServerRequest) -> Result<(), String> {
  let db = open_db(&db_path(&app)?)?;
  let mut identity = ensure_device_identity(&db)?;
  let (vault_key, recovery_secret) = ensure_vault_material(&db)?;
  if !request.device_name.trim().is_empty() {
    set_setting(&db, "device_name", request.device_name.trim())?;
    identity.device_name = request.device_name.trim().to_string();
  }

  let wrapped_vault_key = wrap_vault_key_for_device(&vault_key, &identity.public_key)?;
  let recovery_bundle = encrypt_with_recovery_secret(&recovery_secret, &vault_key)?;
  let response = reqwest::Client::new()
    .post(format!("{}/api/v1/bootstrap", request.server_url.trim_end_matches('/')))
    .json(&serde_json::json!({
      "device_id": identity.device_id,
      "device_name": identity.device_name,
      "device_public_key": identity.public_key,
      "wrapped_vault_key": wrapped_vault_key,
      "recovery_hash": hash_text(&recovery_secret),
      "recovery_bundle": recovery_bundle,
    }))
    .send()
    .await
    .map_err(|error| format!("unable to reach server for bootstrap: {error}"))?
    .error_for_status()
    .map_err(|error| format!("server rejected bootstrap request: {error}"))?
    .json::<ServerBootstrapResponse>()
    .await
    .map_err(|error| format!("unable to parse bootstrap response: {error}"))?;

  set_setting(&db, "server_url", request.server_url.trim())?;
  set_setting(&db, "auth_token", &response.auth_token)?;
  set_setting(&db, "vault_id", &response.vault_id)?;
  db.execute(
    "INSERT INTO trusted_devices (id, device_name, approved_at, is_current)
     VALUES (?1, ?2, ?3, 1)
     ON CONFLICT(id) DO UPDATE SET device_name = excluded.device_name, approved_at = excluded.approved_at, is_current = 1",
    params![identity.device_id, identity.device_name, now_string()],
  )
  .map_err(|error| format!("unable to persist trusted device snapshot: {error}"))?;
  Ok(())
}

#[tauri::command]
pub async fn generate_enrollment_code(app: AppHandle) -> Result<EnrollmentCode, String> {
  let (client, server_url, auth_token) = authorized_client_request(&app).await?;
  let response = client
    .post(format!("{}/api/v1/enrollment-codes", server_url.trim_end_matches('/')))
    .header(AUTHORIZATION, format!("Bearer {auth_token}"))
    .send()
    .await
    .map_err(|error| format!("unable to generate enrollment code: {error}"))?
    .error_for_status()
    .map_err(|error| format!("server rejected enrollment code request: {error}"))?
    .json::<ServerEnrollmentCodeResponse>()
    .await
    .map_err(|error| format!("unable to parse enrollment code response: {error}"))?;
  Ok(EnrollmentCode { code: response.code, expires_at: response.expires_at })
}

#[tauri::command]
pub async fn join_with_enrollment_code(app: AppHandle, request: JoinEnrollmentRequest) -> Result<JoinEnrollmentResponse, String> {
  let db = open_db(&db_path(&app)?)?;
  let mut identity = ensure_device_identity(&db)?;
  if !request.device_name.trim().is_empty() {
    set_setting(&db, "device_name", request.device_name.trim())?;
    identity.device_name = request.device_name.trim().to_string();
  }

  let response = reqwest::Client::new()
    .post(format!("{}/api/v1/enrollments", request.server_url.trim_end_matches('/')))
    .json(&serde_json::json!({
      "code": request.code,
      "device_id": identity.device_id,
      "device_name": identity.device_name,
      "device_public_key": identity.public_key,
    }))
    .send()
    .await
    .map_err(|error| format!("unable to submit enrollment request: {error}"))?
    .error_for_status()
    .map_err(|error| format!("server rejected enrollment request: {error}"))?
    .json::<ServerJoinEnrollmentResponse>()
    .await
    .map_err(|error| format!("unable to parse enrollment response: {error}"))?;

  db.execute(
    "INSERT INTO pending_activations (enrollment_id, activation_secret, server_url, requested_at)
     VALUES (?1, ?2, ?3, ?4)
     ON CONFLICT(enrollment_id) DO UPDATE SET activation_secret = excluded.activation_secret, server_url = excluded.server_url, requested_at = excluded.requested_at",
    params![response.enrollment_id, response.activation_secret, request.server_url.trim(), now_string()],
  )
  .map_err(|error| format!("unable to store pending activation: {error}"))?;

  Ok(JoinEnrollmentResponse {
    enrollment_id: response.enrollment_id,
    expires_at: response.expires_at,
  })
}

#[tauri::command]
pub async fn activate_enrollment(app: AppHandle, enrollment_id: String) -> Result<(), String> {
  let db = open_db(&db_path(&app)?)?;
  let identity = ensure_device_identity(&db)?;
  let pending = db
    .query_row(
      "SELECT activation_secret, server_url FROM pending_activations WHERE enrollment_id = ?1",
      [enrollment_id.as_str()],
      |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )
    .optional()
    .map_err(|error| format!("unable to load pending activation: {error}"))?
    .ok_or_else(|| "pending activation not found".to_string())?;

  let response = reqwest::Client::new()
    .post(format!("{}/api/v1/enrollments/{}/activate", pending.1.trim_end_matches('/'), enrollment_id))
    .json(&serde_json::json!({ "activation_secret": pending.0 }))
    .send()
    .await
    .map_err(|error| format!("unable to activate approved enrollment: {error}"))?
    .error_for_status()
    .map_err(|error| format!("server did not approve the enrollment yet: {error}"))?
    .json::<ActivationResponse>()
    .await
    .map_err(|error| format!("unable to parse activation response: {error}"))?;

  let vault_key = unwrap_vault_key_for_device(&response.wrapped_vault_key, &identity.private_key)?;
  set_setting(&db, "server_url", &pending.1)?;
  set_setting(&db, "auth_token", &response.auth_token)?;
  set_setting(&db, "vault_id", &response.vault_id)?;
  set_setting(&db, "vault_key", &vault_key)?;
  db.execute("DELETE FROM pending_activations WHERE enrollment_id = ?1", [enrollment_id.as_str()])
    .map_err(|error| format!("unable to clear pending activation: {error}"))?;
  Ok(())
}

#[tauri::command]
pub async fn list_trusted_devices(app: AppHandle) -> Result<Vec<TrustedDevice>, String> {
  let db = open_db(&db_path(&app)?)?;
  let current_device_id = get_setting(&db, "device_id")?.unwrap_or_default();
  if let Ok((client, server_url, auth_token)) = authorized_client_request(&app).await {
    let devices = client
      .get(format!("{}/api/v1/devices", server_url.trim_end_matches('/')))
      .header(AUTHORIZATION, format!("Bearer {auth_token}"))
      .send()
      .await
      .map_err(|error| format!("unable to list trusted devices: {error}"))?
      .error_for_status()
      .map_err(|error| format!("server rejected trusted devices request: {error}"))?
      .json::<Vec<ServerTrustedDevice>>()
      .await
      .map_err(|error| format!("unable to parse trusted devices response: {error}"))?;

    for device in devices {
      let is_current = if device.id == current_device_id { 1 } else { 0 };
      db.execute(
        "INSERT INTO trusted_devices (id, device_name, approved_at, revoked_at, is_current)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET device_name = excluded.device_name, approved_at = excluded.approved_at, revoked_at = excluded.revoked_at, is_current = excluded.is_current",
        params![device.id, device.device_name, device.approved_at, device.revoked_at, is_current],
      )
      .map_err(|error| format!("unable to persist trusted device snapshot: {error}"))?;
    }
  }

  let mut statement = db
    .prepare("SELECT id, device_name, approved_at, revoked_at, is_current FROM trusted_devices ORDER BY approved_at DESC")
    .map_err(|error| format!("unable to query trusted devices: {error}"))?;
  let rows = statement
    .query_map([], |row| {
      Ok(TrustedDevice {
        id: row.get(0)?,
        device_name: row.get(1)?,
        approved_at: row.get(2)?,
        revoked_at: row.get(3)?,
        is_current: row.get::<_, i64>(4)? == 1,
      })
    })
    .map_err(|error| format!("unable to map trusted devices: {error}"))?;
  rows.collect::<Result<Vec<_>, _>>().map_err(|error| format!("unable to collect trusted devices: {error}"))
}

#[tauri::command]
pub async fn list_pending_enrollments(app: AppHandle) -> Result<Vec<PendingEnrollment>, String> {
  let (client, server_url, auth_token) = authorized_client_request(&app).await?;
  let response = client
    .get(format!("{}/api/v1/enrollments/pending", server_url.trim_end_matches('/')))
    .header(AUTHORIZATION, format!("Bearer {auth_token}"))
    .send()
    .await
    .map_err(|error| format!("unable to list pending enrollments: {error}"))?
    .error_for_status()
    .map_err(|error| format!("server rejected pending enrollments request: {error}"))?
    .json::<Vec<ServerPendingEnrollment>>()
    .await
    .map_err(|error| format!("unable to parse pending enrollments response: {error}"))?;

  Ok(response
    .into_iter()
    .map(|item| PendingEnrollment { id: item.id, device_name: item.device_name, requested_at: item.requested_at })
    .collect())
}

#[tauri::command]
pub async fn approve_enrollment(app: AppHandle, enrollment_id: String) -> Result<(), String> {
  let db = open_db(&db_path(&app)?)?;
  let vault_key = get_setting(&db, "vault_key")?.ok_or_else(|| "vault key is missing locally".to_string())?;
  let (_, server_url, auth_token) = authorized_client_request(&app).await?;
  let pending = reqwest::Client::new()
    .get(format!("{}/api/v1/enrollments/pending", server_url.trim_end_matches('/')))
    .header(AUTHORIZATION, format!("Bearer {auth_token}"))
    .send()
    .await
    .map_err(|error| format!("unable to reload pending enrollments: {error}"))?
    .error_for_status()
    .map_err(|error| format!("server rejected pending enrollments refresh: {error}"))?
    .json::<Vec<serde_json::Value>>()
    .await
    .map_err(|error| format!("unable to parse pending enrollments refresh: {error}"))?;
  let public_key = pending
    .into_iter()
    .find(|item| item.get("id").and_then(|value| value.as_str()) == Some(enrollment_id.as_str()))
    .and_then(|item| item.get("device_public_key").and_then(|value| value.as_str()).map(str::to_string))
    .ok_or_else(|| "pending enrollment public key not found".to_string())?;
  let wrapped_vault_key = wrap_vault_key_for_device(&vault_key, &public_key)?;
  reqwest::Client::new()
    .post(format!("{}/api/v1/enrollments/{}/approve", server_url.trim_end_matches('/'), enrollment_id))
    .header(AUTHORIZATION, format!("Bearer {auth_token}"))
    .json(&serde_json::json!({ "wrapped_vault_key": wrapped_vault_key }))
    .send()
    .await
    .map_err(|error| format!("unable to approve pending enrollment: {error}"))?
    .error_for_status()
    .map_err(|error| format!("server rejected enrollment approval: {error}"))?;
  Ok(())
}

#[tauri::command]
pub async fn revoke_trusted_device(app: AppHandle, device_id: String) -> Result<(), String> {
  let db = open_db(&db_path(&app)?)?;
  let (_, server_url, auth_token) = authorized_client_request(&app).await?;
  reqwest::Client::new()
    .post(format!("{}/api/v1/devices/{}/revoke", server_url.trim_end_matches('/'), device_id))
    .header(AUTHORIZATION, format!("Bearer {auth_token}"))
    .send()
    .await
    .map_err(|error| format!("unable to revoke trusted device: {error}"))?
    .error_for_status()
    .map_err(|error| format!("server rejected trusted device revocation: {error}"))?;
  db.execute("UPDATE trusted_devices SET revoked_at = ?1 WHERE id = ?2", params![now_string(), device_id])
    .map_err(|error| format!("unable to update local trusted device snapshot: {error}"))?;
  Ok(())
}

#[tauri::command]
pub async fn sync_history_item(app: AppHandle, item_id: String) -> Result<(), String> {
  let db = open_db(&db_path(&app)?)?;
  let identity = ensure_device_identity(&db)?;
  let vault_id = get_setting(&db, "vault_id")?.ok_or_else(|| "vault has not been initialized".to_string())?;
  let vault_key = get_setting(&db, "vault_key")?.ok_or_else(|| "vault key is missing locally".to_string())?;
  let (_, server_url, auth_token) = authorized_client_request(&app).await?;
  let item = db
    .query_row(
      "SELECT content, content_type, content_hash, created_at FROM clipboard_items WHERE id = ?1",
      [item_id.as_str()],
      |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)),
    )
    .optional()
    .map_err(|error| format!("unable to load clipboard item for sync: {error}"))?
    .ok_or_else(|| "clipboard item not found".to_string())?;
  let (ciphertext, nonce) = encrypt_text(&vault_key, &item.0)?;
  let remote_id = Uuid::new_v4().to_string();
  reqwest::Client::new()
    .post(format!("{}/api/v1/clipboard-items", server_url.trim_end_matches('/')))
    .header(AUTHORIZATION, format!("Bearer {auth_token}"))
    .header(CONTENT_TYPE, "application/json")
    .json(&serde_json::json!({
      "id": remote_id,
      "vault_id": vault_id,
      "origin_device_id": identity.device_id,
      "content_type": item.1,
      "ciphertext": ciphertext,
      "nonce": nonce,
      "content_hash": item.2,
      "created_at": item.3,
    }))
    .send()
    .await
    .map_err(|error| format!("unable to sync clipboard item: {error}"))?
    .error_for_status()
    .map_err(|error| format!("server rejected clipboard sync: {error}"))?;
  db.execute(
    "UPDATE clipboard_items SET sync_state = 'synced', last_synced_at = ?1, remote_record_id = ?2 WHERE id = ?3",
    params![now_string(), remote_id, item_id],
  )
  .map_err(|error| format!("unable to update local sync state: {error}"))?;
  Ok(())
}

#[tauri::command]
pub async fn fetch_synced_history(app: AppHandle) -> Result<(), String> {
  let db = open_db(&db_path(&app)?)?;
  let vault_key = get_setting(&db, "vault_key")?.ok_or_else(|| "vault key is missing locally".to_string())?;
  let (_, server_url, auth_token) = authorized_client_request(&app).await?;
  let records = reqwest::Client::new()
    .get(format!("{}/api/v1/clipboard-items", server_url.trim_end_matches('/')))
    .header(AUTHORIZATION, format!("Bearer {auth_token}"))
    .send()
    .await
    .map_err(|error| format!("unable to fetch synced history: {error}"))?
    .error_for_status()
    .map_err(|error| format!("server rejected synced history request: {error}"))?
    .json::<Vec<ServerClipboardItem>>()
    .await
    .map_err(|error| format!("unable to parse synced history response: {error}"))?;
  for record in records {
    let exists = db
      .query_row(
        "SELECT id FROM clipboard_items WHERE remote_record_id = ?1",
        [record.id.as_str()],
        |row| row.get::<_, String>(0),
      )
      .optional()
      .map_err(|error| format!("unable to inspect local record existence: {error}"))?;
    if exists.is_some() {
      continue;
    }
    let content = decrypt_text(&vault_key, &record.ciphertext, &record.nonce)?;
    db.execute(
      "INSERT INTO clipboard_items (
        id, content, content_type, sync_state, created_at, last_synced_at, origin, content_hash, remote_record_id
      ) VALUES (?1, ?2, ?3, 'synced', ?4, ?5, 'cloud', ?6, ?7)",
      params![Uuid::new_v4().to_string(), content, record.content_type, record.created_at, now_string(), record.content_hash, record.id],
    )
    .map_err(|error| format!("unable to store decrypted synced history: {error}"))?;
  }
  Ok(())
}

#[tauri::command]
pub async fn recover_access(app: AppHandle, request: JoinEnrollmentRequest) -> Result<(), String> {
  let db = open_db(&db_path(&app)?)?;
  let mut identity = ensure_device_identity(&db)?;
  if !request.device_name.trim().is_empty() {
    set_setting(&db, "device_name", request.device_name.trim())?;
    identity.device_name = request.device_name.trim().to_string();
  }
  let response = reqwest::Client::new()
    .post(format!("{}/api/v1/recovery/regain-access", request.server_url.trim_end_matches('/')))
    .json(&serde_json::json!({
      "device_id": identity.device_id,
      "device_name": identity.device_name,
      "device_public_key": identity.public_key,
      "recovery_secret": request.code,
    }))
    .send()
    .await
    .map_err(|error| format!("unable to attempt recovery: {error}"))?
    .error_for_status()
    .map_err(|error| format!("server rejected recovery request: {error}"))?
    .json::<serde_json::Value>()
    .await
    .map_err(|error| format!("unable to parse recovery response: {error}"))?;
  let bundle = response.get("recovery_bundle").and_then(|value| value.as_str()).ok_or_else(|| "recovery bundle missing from response".to_string())?;
  let auth_token = response.get("auth_token").and_then(|value| value.as_str()).ok_or_else(|| "recovery auth token missing from response".to_string())?;
  let vault_id = response.get("vault_id").and_then(|value| value.as_str()).ok_or_else(|| "recovery vault id missing from response".to_string())?;
  let vault_key = decrypt_with_recovery_secret(&request.code, bundle)?;
  set_setting(&db, "server_url", request.server_url.trim())?;
  set_setting(&db, "auth_token", auth_token)?;
  set_setting(&db, "vault_id", vault_id)?;
  set_setting(&db, "vault_key", &vault_key)?;
  Ok(())
}
