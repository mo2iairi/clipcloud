use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AppSnapshot {
  pub device: DeviceProfile,
  pub items: Vec<ClipboardHistoryItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeviceProfile {
  pub device_id: String,
  pub device_name: String,
  pub capture_mode: String,
  pub server_url: Option<String>,
  pub is_connected_to_server: bool,
  pub vault_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ClipboardHistoryItem {
  pub id: String,
  pub content: String,
  pub content_type: String,
  pub sync_state: String,
  pub created_at: String,
  pub last_synced_at: Option<String>,
  pub origin: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TrustedDevice {
  pub id: String,
  pub device_name: String,
  pub approved_at: String,
  pub revoked_at: Option<String>,
  pub is_current: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PendingEnrollment {
  pub id: String,
  pub device_name: String,
  pub requested_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct EnrollmentCode {
  pub code: String,
  pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BootstrapServerRequest {
  pub server_url: String,
  pub device_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct JoinEnrollmentRequest {
  pub server_url: String,
  pub device_name: String,
  pub code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct JoinEnrollmentResponse {
  pub enrollment_id: String,
  pub expires_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerBootstrapResponse {
  pub vault_id: String,
  pub auth_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerEnrollmentCodeResponse {
  pub code: String,
  pub expires_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerPendingEnrollment {
  pub id: String,
  pub device_name: String,
  pub requested_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerTrustedDevice {
  pub id: String,
  pub device_name: String,
  pub approved_at: String,
  pub revoked_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerClipboardItem {
  pub id: String,
  pub content_type: String,
  pub ciphertext: String,
  pub nonce: String,
  pub content_hash: String,
  pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerJoinEnrollmentResponse {
  pub enrollment_id: String,
  pub expires_at: String,
  pub activation_secret: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActivationResponse {
  pub auth_token: String,
  pub vault_id: String,
  pub wrapped_vault_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeviceIdentity {
  pub device_id: String,
  pub device_name: String,
  pub public_key: String,
  pub private_key: String,
}
