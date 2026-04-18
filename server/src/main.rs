use axum::{
  extract::{Path, Query, State},
  http::{header::AUTHORIZATION, HeaderMap, StatusCode},
  response::IntoResponse,
  routing::{get, post},
  Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool, Row};
use std::{collections::HashMap, env, net::SocketAddr};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use tower_http::trace::TraceLayer;
use tracing::Level;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
  pool: PgPool,
}

#[derive(Debug)]
struct AppError {
  status: StatusCode,
  message: String,
}

impl AppError {
  fn new(status: StatusCode, message: impl Into<String>) -> Self {
    Self { status, message: message.into() }
  }
}

impl IntoResponse for AppError {
  fn into_response(self) -> axum::response::Response {
    (self.status, Json(serde_json::json!({ "error": self.message }))).into_response()
  }
}

impl From<sqlx::Error> for AppError {
  fn from(error: sqlx::Error) -> Self {
    Self::new(StatusCode::INTERNAL_SERVER_ERROR, format!("database error: {error}"))
  }
}

impl std::fmt::Display for AppError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl std::error::Error for AppError {}

#[derive(Serialize)]
struct HealthResponse {
  status: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct VaultStatus {
  initialized: bool,
  vault_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct BootstrapRequest {
  device_id: String,
  device_name: String,
  device_public_key: String,
  wrapped_vault_key: String,
  recovery_hash: String,
  recovery_bundle: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct BootstrapResponse {
  vault_id: String,
  auth_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct EnrollmentRequest {
  code: String,
  device_id: String,
  device_name: String,
  device_public_key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EnrollmentResponse {
  enrollment_id: String,
  expires_at: String,
  activation_secret: String,
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct EnrollmentView {
  id: String,
  device_name: String,
  device_public_key: String,
  requested_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct ApproveEnrollmentRequest {
  wrapped_vault_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct ActivateEnrollmentRequest {
  activation_secret: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ActivateEnrollmentResponse {
  auth_token: String,
  vault_id: String,
  wrapped_vault_key: String,
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct DeviceView {
  id: String,
  device_name: String,
  approved_at: String,
  revoked_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct ClipboardItemRequest {
  id: String,
  vault_id: String,
  origin_device_id: String,
  content_type: String,
  ciphertext: String,
  nonce: String,
  content_hash: String,
  created_at: String,
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct ClipboardItemView {
  id: String,
  origin_device_id: String,
  content_type: String,
  ciphertext: String,
  nonce: String,
  content_hash: String,
  created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct RecoveryRequest {
  device_id: String,
  device_name: String,
  device_public_key: String,
  recovery_secret: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct RecoveryResponse {
  auth_token: String,
  vault_id: String,
  recovery_bundle: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EnrollmentCodeResponse {
  code: String,
  expires_at: String,
}

fn hash_token(token: &str) -> String {
  BASE64.encode(Sha256::digest(token.as_bytes()))
}

fn random_token(length: usize) -> String {
  rand::thread_rng()
    .sample_iter(&Alphanumeric)
    .take(length)
    .map(char::from)
    .collect()
}

async fn initialize_schema(pool: &PgPool) -> Result<(), AppError> {
  let statements = [
    "
    CREATE TABLE IF NOT EXISTS vaults (
      id UUID PRIMARY KEY,
      recovery_hash TEXT NOT NULL,
      recovery_bundle TEXT NOT NULL,
      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    ",
    "
    CREATE TABLE IF NOT EXISTS devices (
      id TEXT PRIMARY KEY,
      vault_id UUID NOT NULL REFERENCES vaults(id),
      device_name TEXT NOT NULL,
      device_public_key TEXT NOT NULL,
      wrapped_vault_key TEXT NOT NULL,
      auth_token_hash TEXT NOT NULL,
      approved_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      revoked_at TIMESTAMPTZ NULL
    )
    ",
    "
    CREATE TABLE IF NOT EXISTS enrollment_codes (
      id UUID PRIMARY KEY,
      vault_id UUID NOT NULL REFERENCES vaults(id),
      code_hash TEXT NOT NULL,
      issued_by_device_id TEXT NOT NULL REFERENCES devices(id),
      expires_at TIMESTAMPTZ NOT NULL,
      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    ",
    "
    CREATE TABLE IF NOT EXISTS pending_enrollments (
      id UUID PRIMARY KEY,
      vault_id UUID NOT NULL REFERENCES vaults(id),
      device_id TEXT NOT NULL,
      device_name TEXT NOT NULL,
      device_public_key TEXT NOT NULL,
      activation_secret_hash TEXT NOT NULL,
      expires_at TIMESTAMPTZ NOT NULL,
      approved_at TIMESTAMPTZ NULL,
      approved_by_device_id TEXT NULL REFERENCES devices(id),
      wrapped_vault_key TEXT NULL,
      auth_token_hash TEXT NULL,
      requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    ",
    "
    CREATE TABLE IF NOT EXISTS clipboard_items (
      id TEXT PRIMARY KEY,
      vault_id UUID NOT NULL REFERENCES vaults(id),
      origin_device_id TEXT NOT NULL REFERENCES devices(id),
      content_type TEXT NOT NULL,
      ciphertext TEXT NOT NULL,
      nonce TEXT NOT NULL,
      content_hash TEXT NOT NULL,
      created_at TIMESTAMPTZ NOT NULL
    )
    ",
  ];

  for statement in statements {
    sqlx::query(statement).execute(pool).await?;
  }

  Ok(())
}

async fn get_single_vault_id(pool: &PgPool) -> Result<Option<Uuid>, AppError> {
  let row = sqlx::query("SELECT id FROM vaults LIMIT 1").fetch_optional(pool).await?;
  row.map(|row| row.try_get::<Uuid, _>("id"))
    .transpose()
    .map_err(|error| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to read vault id: {error}")))
}

async fn authorize(headers: &HeaderMap, pool: &PgPool) -> Result<(Uuid, String), AppError> {
  let header = headers
    .get(AUTHORIZATION)
    .and_then(|value| value.to_str().ok())
    .ok_or_else(|| AppError::new(StatusCode::UNAUTHORIZED, "missing bearer token"))?;
  let token = header
    .strip_prefix("Bearer ")
    .ok_or_else(|| AppError::new(StatusCode::UNAUTHORIZED, "invalid bearer token format"))?;
  let row = sqlx::query("SELECT vault_id, id FROM devices WHERE auth_token_hash = $1 AND revoked_at IS NULL LIMIT 1")
    .bind(hash_token(token))
    .fetch_optional(pool)
    .await?;
  let row = row.ok_or_else(|| AppError::new(StatusCode::UNAUTHORIZED, "unknown bearer token"))?;
  let vault_id = row.try_get::<Uuid, _>("vault_id").map_err(|error| {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode vault id: {error}"))
  })?;
  let device_id = row.try_get::<String, _>("id").map_err(|error| {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode device id: {error}"))
  })?;
  Ok((vault_id, device_id))
}

async fn health() -> Json<HealthResponse> {
  Json(HealthResponse { status: "ok" })
}

async fn vault_status(State(state): State<AppState>) -> Result<Json<VaultStatus>, AppError> {
  let vault_id = get_single_vault_id(&state.pool).await?;
  Ok(Json(VaultStatus {
    initialized: vault_id.is_some(),
    vault_id: vault_id.map(|value| value.to_string()),
  }))
}

async fn bootstrap(State(state): State<AppState>, Json(request): Json<BootstrapRequest>) -> Result<Json<BootstrapResponse>, AppError> {
  if get_single_vault_id(&state.pool).await?.is_some() {
    return Err(AppError::new(StatusCode::CONFLICT, "vault has already been initialized"));
  }
  let vault_id = Uuid::new_v4();
  let auth_token = random_token(48);
  let mut tx = state.pool.begin().await?;
  sqlx::query("INSERT INTO vaults (id, recovery_hash, recovery_bundle) VALUES ($1, $2, $3)")
    .bind(vault_id)
    .bind(request.recovery_hash)
    .bind(request.recovery_bundle)
    .execute(&mut *tx)
    .await?;
  sqlx::query(
    "INSERT INTO devices (id, vault_id, device_name, device_public_key, wrapped_vault_key, auth_token_hash)
     VALUES ($1, $2, $3, $4, $5, $6)",
  )
  .bind(request.device_id)
  .bind(vault_id)
  .bind(request.device_name)
  .bind(request.device_public_key)
  .bind(request.wrapped_vault_key)
  .bind(hash_token(&auth_token))
  .execute(&mut *tx)
  .await?;
  tx.commit().await?;
  Ok(Json(BootstrapResponse { vault_id: vault_id.to_string(), auth_token }))
}

async fn create_enrollment_code(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<EnrollmentCodeResponse>, AppError> {
  let (vault_id, device_id) = authorize(&headers, &state.pool).await?;
  let code = random_token(6).to_uppercase();
  let expires_at = OffsetDateTime::now_utc() + Duration::minutes(5);
  sqlx::query(
    "INSERT INTO enrollment_codes (id, vault_id, code_hash, issued_by_device_id, expires_at)
     VALUES ($1, $2, $3, $4, $5)",
  )
  .bind(Uuid::new_v4())
  .bind(vault_id)
  .bind(hash_token(&code))
  .bind(device_id)
  .bind(expires_at)
  .execute(&state.pool)
  .await?;
  Ok(Json(EnrollmentCodeResponse {
    code,
    expires_at: expires_at.format(&Rfc3339).unwrap_or_else(|_| expires_at.unix_timestamp().to_string()),
  }))
}

async fn request_enrollment(State(state): State<AppState>, Json(request): Json<EnrollmentRequest>) -> Result<Json<EnrollmentResponse>, AppError> {
  let row = sqlx::query("SELECT vault_id, expires_at FROM enrollment_codes WHERE code_hash = $1 ORDER BY created_at DESC LIMIT 1")
    .bind(hash_token(&request.code.to_uppercase()))
    .fetch_optional(&state.pool)
    .await?;
  let row = row.ok_or_else(|| AppError::new(StatusCode::BAD_REQUEST, "enrollment code is invalid"))?;
  let expires_at = row.try_get::<OffsetDateTime, _>("expires_at").map_err(|error| {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode enrollment code expiry: {error}"))
  })?;
  if expires_at < OffsetDateTime::now_utc() {
    return Err(AppError::new(StatusCode::BAD_REQUEST, "enrollment code has expired"));
  }
  let vault_id = row.try_get::<Uuid, _>("vault_id").map_err(|error| {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode enrollment vault id: {error}"))
  })?;
  let enrollment_id = Uuid::new_v4();
  let activation_secret = random_token(32);
  sqlx::query(
    "INSERT INTO pending_enrollments (
      id, vault_id, device_id, device_name, device_public_key, activation_secret_hash, expires_at
     ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
  )
  .bind(enrollment_id)
  .bind(vault_id)
  .bind(request.device_id)
  .bind(request.device_name)
  .bind(request.device_public_key)
  .bind(hash_token(&activation_secret))
  .bind(expires_at)
  .execute(&state.pool)
  .await?;
  Ok(Json(EnrollmentResponse {
    enrollment_id: enrollment_id.to_string(),
    expires_at: expires_at.format(&Rfc3339).unwrap_or_else(|_| expires_at.unix_timestamp().to_string()),
    activation_secret,
  }))
}

async fn list_pending_enrollments(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Vec<EnrollmentView>>, AppError> {
  let (vault_id, _) = authorize(&headers, &state.pool).await?;
  let enrollments = sqlx::query_as::<_, EnrollmentView>(
    "SELECT id::text AS id, device_name, device_public_key,
            to_char(requested_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS requested_at
     FROM pending_enrollments
     WHERE vault_id = $1 AND approved_at IS NULL AND expires_at >= NOW()
     ORDER BY requested_at DESC",
  )
  .bind(vault_id)
  .fetch_all(&state.pool)
  .await?;
  Ok(Json(enrollments))
}

async fn approve_enrollment(
  State(state): State<AppState>,
  headers: HeaderMap,
  Path(enrollment_id): Path<String>,
  Json(request): Json<ApproveEnrollmentRequest>,
) -> Result<StatusCode, AppError> {
  let (vault_id, approver_device_id) = authorize(&headers, &state.pool).await?;
  let enrollment_uuid = Uuid::parse_str(&enrollment_id)
    .map_err(|error| AppError::new(StatusCode::BAD_REQUEST, format!("invalid enrollment identifier: {error}")))?;
  let pending = sqlx::query(
    "SELECT device_id, device_name, device_public_key
     FROM pending_enrollments
     WHERE id = $1 AND vault_id = $2 AND approved_at IS NULL AND expires_at >= NOW()",
  )
  .bind(enrollment_uuid)
  .bind(vault_id)
  .fetch_optional(&state.pool)
  .await?;
  let pending = pending.ok_or_else(|| AppError::new(StatusCode::BAD_REQUEST, "pending enrollment not found"))?;
  let auth_token = random_token(48);
  let mut tx = state.pool.begin().await?;
  sqlx::query(
    "UPDATE pending_enrollments
     SET approved_at = NOW(), approved_by_device_id = $1, wrapped_vault_key = $2, auth_token_hash = $3
     WHERE id = $4",
  )
  .bind(approver_device_id)
  .bind(&request.wrapped_vault_key)
  .bind(hash_token(&auth_token))
  .bind(enrollment_uuid)
  .execute(&mut *tx)
  .await?;
  sqlx::query(
    "INSERT INTO devices (id, vault_id, device_name, device_public_key, wrapped_vault_key, auth_token_hash, approved_at)
     VALUES ($1, $2, $3, $4, $5, $6, NOW())",
  )
  .bind(pending.try_get::<String, _>("device_id").map_err(|error| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode pending device id: {error}")))?)
  .bind(vault_id)
  .bind(pending.try_get::<String, _>("device_name").map_err(|error| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode pending device name: {error}")))?)
  .bind(pending.try_get::<String, _>("device_public_key").map_err(|error| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode pending device key: {error}")))?)
  .bind(request.wrapped_vault_key)
  .bind(hash_token(&auth_token))
  .execute(&mut *tx)
  .await?;
  tx.commit().await?;
  Ok(StatusCode::NO_CONTENT)
}

async fn activate_enrollment(
  State(state): State<AppState>,
  Path(enrollment_id): Path<String>,
  Json(request): Json<ActivateEnrollmentRequest>,
) -> Result<Json<ActivateEnrollmentResponse>, AppError> {
  let enrollment_uuid = Uuid::parse_str(&enrollment_id)
    .map_err(|error| AppError::new(StatusCode::BAD_REQUEST, format!("invalid enrollment identifier: {error}")))?;
  let row = sqlx::query(
    "SELECT vault_id, wrapped_vault_key, device_id, activation_secret_hash
     FROM pending_enrollments
     WHERE id = $1 AND approved_at IS NOT NULL AND expires_at >= NOW()",
  )
  .bind(enrollment_uuid)
  .fetch_optional(&state.pool)
  .await?;
  let row = row.ok_or_else(|| AppError::new(StatusCode::BAD_REQUEST, "enrollment is not approved yet"))?;
  let expected_secret_hash = row.try_get::<String, _>("activation_secret_hash").map_err(|error| {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode activation secret hash: {error}"))
  })?;
  if expected_secret_hash != hash_token(&request.activation_secret) {
    return Err(AppError::new(StatusCode::UNAUTHORIZED, "activation secret does not match"));
  }
  let auth_token = random_token(48);
  let device_id = row.try_get::<String, _>("device_id").map_err(|error| {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode activation device id: {error}"))
  })?;
  sqlx::query("UPDATE devices SET auth_token_hash = $1 WHERE id = $2")
    .bind(hash_token(&auth_token))
    .bind(device_id)
    .execute(&state.pool)
    .await?;
  let vault_id = row.try_get::<Uuid, _>("vault_id").map_err(|error| {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode activation vault id: {error}"))
  })?;
  let wrapped_vault_key = row.try_get::<String, _>("wrapped_vault_key").map_err(|error| {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode wrapped vault key: {error}"))
  })?;
  Ok(Json(ActivateEnrollmentResponse {
    auth_token,
    vault_id: vault_id.to_string(),
    wrapped_vault_key,
  }))
}

async fn list_devices(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Vec<DeviceView>>, AppError> {
  let (vault_id, _) = authorize(&headers, &state.pool).await?;
  let devices = sqlx::query_as::<_, DeviceView>(
    "SELECT id, device_name,
            to_char(approved_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS approved_at,
            CASE WHEN revoked_at IS NULL THEN NULL ELSE to_char(revoked_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') END AS revoked_at
     FROM devices
     WHERE vault_id = $1
     ORDER BY approved_at DESC",
  )
  .bind(vault_id)
  .fetch_all(&state.pool)
  .await?;
  Ok(Json(devices))
}

async fn revoke_device(State(state): State<AppState>, headers: HeaderMap, Path(device_id): Path<String>) -> Result<StatusCode, AppError> {
  let (vault_id, current_device_id) = authorize(&headers, &state.pool).await?;
  if current_device_id == device_id {
    return Err(AppError::new(StatusCode::BAD_REQUEST, "current device cannot revoke itself"));
  }
  sqlx::query("UPDATE devices SET revoked_at = NOW() WHERE id = $1 AND vault_id = $2")
    .bind(device_id)
    .bind(vault_id)
    .execute(&state.pool)
    .await?;
  Ok(StatusCode::NO_CONTENT)
}

async fn store_clipboard_item(State(state): State<AppState>, headers: HeaderMap, Json(request): Json<ClipboardItemRequest>) -> Result<StatusCode, AppError> {
  let (vault_id, _) = authorize(&headers, &state.pool).await?;
  let payload_vault_id = Uuid::parse_str(&request.vault_id)
    .map_err(|error| AppError::new(StatusCode::BAD_REQUEST, format!("invalid vault identifier: {error}")))?;
  if payload_vault_id != vault_id {
    return Err(AppError::new(StatusCode::UNAUTHORIZED, "clipboard record targets the wrong vault"));
  }
  sqlx::query(
    "INSERT INTO clipboard_items (id, vault_id, origin_device_id, content_type, ciphertext, nonce, content_hash, created_at)
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
     ON CONFLICT (id) DO NOTHING",
  )
  .bind(request.id)
  .bind(vault_id)
  .bind(request.origin_device_id)
  .bind(request.content_type)
  .bind(request.ciphertext)
  .bind(request.nonce)
  .bind(request.content_hash)
  .bind(OffsetDateTime::parse(&request.created_at, &Rfc3339).map_err(|error| {
    AppError::new(StatusCode::BAD_REQUEST, format!("invalid clipboard timestamp: {error}"))
  })?)
  .execute(&state.pool)
  .await?;
  Ok(StatusCode::NO_CONTENT)
}

async fn list_clipboard_items(
  State(state): State<AppState>,
  headers: HeaderMap,
  Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Vec<ClipboardItemView>>, AppError> {
  let (vault_id, _) = authorize(&headers, &state.pool).await?;
  let items = if let Some(origin_device_id) = query.get("origin_device_id") {
    sqlx::query_as::<_, ClipboardItemView>(
      "SELECT id, origin_device_id, content_type, ciphertext, nonce, content_hash,
              to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at
       FROM clipboard_items
       WHERE vault_id = $1 AND origin_device_id = $2
       ORDER BY created_at DESC",
    )
    .bind(vault_id)
    .bind(origin_device_id)
    .fetch_all(&state.pool)
    .await?
  } else {
    sqlx::query_as::<_, ClipboardItemView>(
      "SELECT id, origin_device_id, content_type, ciphertext, nonce, content_hash,
              to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at
       FROM clipboard_items
       WHERE vault_id = $1
       ORDER BY created_at DESC",
    )
    .bind(vault_id)
    .fetch_all(&state.pool)
    .await?
  };
  Ok(Json(items))
}

async fn recover_access(State(state): State<AppState>, Json(request): Json<RecoveryRequest>) -> Result<Json<RecoveryResponse>, AppError> {
  let row = sqlx::query("SELECT id, recovery_hash, recovery_bundle FROM vaults LIMIT 1")
    .fetch_optional(&state.pool)
    .await?;
  let row = row.ok_or_else(|| AppError::new(StatusCode::BAD_REQUEST, "vault is not initialized"))?;
  let recovery_hash = row.try_get::<String, _>("recovery_hash").map_err(|error| {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode recovery hash: {error}"))
  })?;
  if recovery_hash != hash_token(&request.recovery_secret) {
    return Err(AppError::new(StatusCode::UNAUTHORIZED, "recovery secret does not match"));
  }
  let vault_id = row.try_get::<Uuid, _>("id").map_err(|error| {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode recovery vault id: {error}"))
  })?;
  let recovery_bundle = row.try_get::<String, _>("recovery_bundle").map_err(|error| {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("unable to decode recovery bundle: {error}"))
  })?;
  let auth_token = random_token(48);
  sqlx::query(
    "INSERT INTO devices (id, vault_id, device_name, device_public_key, wrapped_vault_key, auth_token_hash, approved_at)
     VALUES ($1, $2, $3, $4, '', $5, NOW())
     ON CONFLICT (id) DO UPDATE SET device_name = excluded.device_name, device_public_key = excluded.device_public_key, auth_token_hash = excluded.auth_token_hash, revoked_at = NULL",
  )
  .bind(request.device_id)
  .bind(vault_id)
  .bind(request.device_name)
  .bind(request.device_public_key)
  .bind(hash_token(&auth_token))
  .execute(&state.pool)
  .await?;
  Ok(Json(RecoveryResponse {
    auth_token,
    vault_id: vault_id.to_string(),
    recovery_bundle,
  }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let _ = dotenvy::from_filename(".env");

  tracing_subscriber::fmt()
    .with_max_level(Level::INFO)
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();

  let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/clipcloud".to_string());
  let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8787".to_string());

  let pool = PgPoolOptions::new().max_connections(10).connect(&database_url).await?;
  initialize_schema(&pool).await?;

  let app = Router::new()
    .route("/health", get(health))
    .route("/api/v1/vault/status", get(vault_status))
    .route("/api/v1/bootstrap", post(bootstrap))
    .route("/api/v1/enrollment-codes", post(create_enrollment_code))
    .route("/api/v1/enrollments", post(request_enrollment))
    .route("/api/v1/enrollments/pending", get(list_pending_enrollments))
    .route("/api/v1/enrollments/{enrollment_id}/approve", post(approve_enrollment))
    .route("/api/v1/enrollments/{enrollment_id}/activate", post(activate_enrollment))
    .route("/api/v1/devices", get(list_devices))
    .route("/api/v1/devices/{device_id}/revoke", post(revoke_device))
    .route("/api/v1/clipboard-items", post(store_clipboard_item).get(list_clipboard_items))
    .route("/api/v1/recovery/regain-access", post(recover_access))
    .layer(TraceLayer::new_for_http())
    .with_state(AppState { pool });

  let address: SocketAddr = bind_address.parse()?;
  tracing::info!("clipcloud server listening on {address}");
  let listener = tokio::net::TcpListener::bind(address).await?;
  axum::serve(listener, app).await?;
  Ok(())
}
