# ClipCloud

ClipCloud is a private, single-user clipboard vault built around a local-first Tauri client and a self-hosted Rust + PostgreSQL sync service.

## Current shape

- Desktop client captures clipboard text into a local SQLite database with no server requirement.
- The history UI shows local-only versus synced state and lets the user explicitly choose which items to upload.
- The sync service is single-owner: one deployment boots one vault, not multiple accounts.
- Synced clipboard payloads are encrypted client-side before upload.
- New devices join through a short-lived enrollment code plus approval from an already trusted device.
- Recovery uses a recovery secret and a recovery bundle stored on the server so the server does not need plaintext clipboard content.

## Repository layout

- `client/`: React + Tauri application, local SQLite store, clipboard capture, sync and device commands.
- `server/`: Axum service with PostgreSQL persistence for vault bootstrap, device trust, enrollment, and encrypted clipboard records.
- `openspec/changes/add-private-clipboard-vault/`: proposal, design, specs, and task tracking for the current change.

## Running the server

Set a PostgreSQL connection string and start the API:

```powershell
$env:DATABASE_URL="postgres://postgres:postgres@127.0.0.1:5432/clipcloud"
cd server
cargo run
```

The default bind address is `127.0.0.1:8787`. Override it with `BIND_ADDRESS` if needed.

## Running the client

```powershell
cd client
npm install
npm run build
cargo tauri dev
```

On desktop, the Tauri backend polls the system clipboard and inserts new text entries into local history.

## Bootstrap and enrollment flow

1. Start the server.
2. Open the client on the first device and use `Bootstrap personal server`.
3. On a second device, enter the same server URL plus the short-lived enrollment code and choose `Request enrollment`.
4. Approve the pending device from an already trusted device.
5. On the new device, choose `Activate approved device` to receive its wrapped vault key and auth token.

## Privacy model

- Clipboard history is local by default.
- The user must explicitly choose `Sync to cloud` per history item.
- The server stores ciphertext, wrapped key material, and trust metadata.
- The current implementation keeps vault key material only on trusted devices and encrypted recovery storage.

## Recovery

The client generates a recovery secret during initial vault creation. The server stores an encrypted recovery bundle and verifies the recovery secret only to re-establish a trusted device. Without a trusted device or the recovery secret, the server alone cannot restore clipboard plaintext.

## Verification completed

- `client`: `npm run build`
- `client/src-tauri`: `cargo check`
- `server`: `cargo check`

## Remaining validation

- Full end-to-end device enrollment and sync should still be exercised against a live PostgreSQL instance.
- Offline and no-server behavior is implemented in the local client path but still needs explicit acceptance validation.
