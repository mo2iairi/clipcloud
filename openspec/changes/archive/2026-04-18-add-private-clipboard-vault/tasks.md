## 1. Client Local-First Foundation

- [x] 1.1 Set up the Tauri client workspace for shared desktop and mobile application code
- [x] 1.2 Define the local SQLite schema for clipboard history, device metadata, sync state, and trusted device records
- [x] 1.3 Implement local clipboard capture and history persistence for supported platforms
- [x] 1.4 Build the history UI with per-item local-only and synced state indicators
- [x] 1.5 Add the explicit user action that marks a history item for encrypted cloud sync

## 2. Server Single-Owner Foundation

- [x] 2.1 Set up the Rust HTTP service and PostgreSQL schema for a single-owner vault deployment
- [x] 2.2 Implement bootstrap initialization that creates exactly one owner vault and records the first trusted device
- [x] 2.3 Implement encrypted clipboard item storage APIs using ciphertext plus sync metadata only
- [x] 2.4 Implement history listing APIs that filter by non-sensitive metadata such as time, type, and origin device
- [x] 2.5 Prevent second-owner or general multi-user registration flows on an initialized server

## 3. Encryption and Device Trust

- [x] 3.1 Implement client-side key generation for device identity and vault encryption
- [x] 3.2 Implement the short-lived enrollment code flow that creates a pending join request
- [x] 3.3 Implement trusted-device approval that wraps vault key material for a pending device public key
- [x] 3.4 Implement trusted device listing and revocation flows across client and server
- [x] 3.5 Implement the recovery secret flow for restoring vault access when no trusted device remains

## 4. Sync Integration and Validation

- [x] 4.1 Integrate selective encrypted item upload from the client to the server
- [x] 4.2 Implement trusted-device download and local decryption of synced clipboard items
- [ ] 4.3 Verify that untrusted devices cannot obtain usable key material or decrypt synced items
- [ ] 4.4 Validate local-first behavior when no server is configured or the network is unavailable
- [x] 4.5 Document deployment, bootstrap, recovery, and privacy guarantees for self-hosted users
