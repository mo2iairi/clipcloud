use age::{x25519, Decryptor, Encryptor};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chacha20poly1305::{
  aead::{Aead, KeyInit},
  ChaCha20Poly1305, Key, Nonce,
};
use rand::{distributions::Alphanumeric, rngs::OsRng, Rng, RngCore};
use sha2::{Digest, Sha256};
use std::{io::Write, iter};

pub fn hash_text(value: &str) -> String {
  BASE64.encode(Sha256::digest(value.as_bytes()))
}

pub fn random_secret() -> String {
  rand::thread_rng()
    .sample_iter(&Alphanumeric)
    .take(32)
    .map(char::from)
    .collect()
}

pub fn encrypt_text(vault_key_b64: &str, plaintext: &str) -> Result<(String, String), String> {
  let vault_key = BASE64
    .decode(vault_key_b64)
    .map_err(|error| format!("unable to decode vault key: {error}"))?;
  let cipher = ChaCha20Poly1305::new(Key::from_slice(&vault_key));
  let mut nonce = [0_u8; 12];
  OsRng.fill_bytes(&mut nonce);
  let ciphertext = cipher
    .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
    .map_err(|error| format!("unable to encrypt clipboard content: {error}"))?;
  Ok((BASE64.encode(ciphertext), BASE64.encode(nonce)))
}

pub fn decrypt_text(vault_key_b64: &str, ciphertext_b64: &str, nonce_b64: &str) -> Result<String, String> {
  let vault_key = BASE64
    .decode(vault_key_b64)
    .map_err(|error| format!("unable to decode vault key: {error}"))?;
  let nonce = BASE64
    .decode(nonce_b64)
    .map_err(|error| format!("unable to decode nonce: {error}"))?;
  let ciphertext = BASE64
    .decode(ciphertext_b64)
    .map_err(|error| format!("unable to decode ciphertext: {error}"))?;
  let cipher = ChaCha20Poly1305::new(Key::from_slice(&vault_key));
  let plaintext = cipher
    .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
    .map_err(|error| format!("unable to decrypt clipboard content: {error}"))?;
  String::from_utf8(plaintext).map_err(|error| format!("invalid utf-8 clipboard content: {error}"))
}

pub fn encrypt_with_recovery_secret(secret: &str, payload: &str) -> Result<String, String> {
  let derived = Sha256::digest(secret.as_bytes());
  let cipher = ChaCha20Poly1305::new(Key::from_slice(&derived));
  let mut nonce = [0_u8; 12];
  OsRng.fill_bytes(&mut nonce);
  let ciphertext = cipher
    .encrypt(Nonce::from_slice(&nonce), payload.as_bytes())
    .map_err(|error| format!("unable to encrypt recovery bundle: {error}"))?;
  Ok(format!("{}:{}", BASE64.encode(nonce), BASE64.encode(ciphertext)))
}

pub fn decrypt_with_recovery_secret(secret: &str, payload: &str) -> Result<String, String> {
  let (nonce_b64, ciphertext_b64) = payload
    .split_once(':')
    .ok_or_else(|| "invalid recovery bundle".to_string())?;
  let derived = Sha256::digest(secret.as_bytes());
  let cipher = ChaCha20Poly1305::new(Key::from_slice(&derived));
  let nonce = BASE64
    .decode(nonce_b64)
    .map_err(|error| format!("unable to decode recovery nonce: {error}"))?;
  let ciphertext = BASE64
    .decode(ciphertext_b64)
    .map_err(|error| format!("unable to decode recovery ciphertext: {error}"))?;
  let plaintext = cipher
    .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
    .map_err(|error| format!("unable to decrypt recovery bundle: {error}"))?;
  String::from_utf8(plaintext).map_err(|error| format!("invalid recovery payload: {error}"))
}

pub fn wrap_vault_key_for_device(vault_key: &str, recipient_public_key: &str) -> Result<String, String> {
  let recipient: x25519::Recipient = recipient_public_key
    .parse()
    .map_err(|error| format!("unable to parse recipient key: {error}"))?;
  let encryptor = Encryptor::with_recipients(iter::once(&recipient as &dyn age::Recipient))
    .map_err(|error| format!("unable to construct recipient encryptor: {error}"))?;
  let mut output = Vec::new();
  let mut writer = encryptor
    .wrap_output(&mut output)
    .map_err(|error| format!("unable to wrap vault key: {error}"))?;
  writer
    .write_all(vault_key.as_bytes())
    .map_err(|error| format!("unable to write wrapped vault key: {error}"))?;
  writer
    .finish()
    .map_err(|error| format!("unable to finish wrapped vault key: {error}"))?;
  Ok(BASE64.encode(output))
}

pub fn unwrap_vault_key_for_device(wrapped_payload_b64: &str, private_key_text: &str) -> Result<String, String> {
  let wrapped = BASE64
    .decode(wrapped_payload_b64)
    .map_err(|error| format!("unable to decode wrapped vault key: {error}"))?;
  let identity: x25519::Identity = private_key_text
    .parse()
    .map_err(|error| format!("unable to parse local private key: {error}"))?;
  let decryptor = Decryptor::new(wrapped.as_slice()).map_err(|error| format!("unable to open wrapped vault key: {error}"))?;
  let mut reader = decryptor
    .decrypt(iter::once(&identity as &dyn age::Identity))
    .map_err(|error| format!("unable to decrypt wrapped vault key: {error}"))?;
  let mut output = Vec::new();
  std::io::copy(&mut reader, &mut output).map_err(|error| format!("unable to read wrapped vault key: {error}"))?;
  String::from_utf8(output).map_err(|error| format!("invalid wrapped vault key payload: {error}"))
}
