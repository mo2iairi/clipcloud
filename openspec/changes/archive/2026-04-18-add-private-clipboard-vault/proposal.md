## Why

The product goal is a private clipboard tool for a single person across multiple devices, with optional self-hosted cloud sync and source code open for review. Existing consumer clipboard products usually assume either a centralized service model or plaintext server access, which conflicts with the requirement that the server must not be able to read user clipboard content.

## What Changes

- Introduce a local-first clipboard history client where captured content is stored in a local embedded database by default.
- Add an encrypted cloud sync model where users explicitly choose clipboard history items to sync to their personal server.
- Add a single-owner server model that binds one deployed server instance to one personal vault rather than supporting multi-user accounts.
- Add cross-device trust enrollment so a new device can only join the vault after approval from an already unlocked device using a short-lived enrollment code.
- Define a shared client model across desktop and mobile, with platform-specific capability differences handled as implementation details rather than separate product roles.
- Define recovery expectations for end-to-end encrypted sync so users can regain access without requiring the server to decrypt stored data.

## Capabilities

### New Capabilities
- `local-clipboard-history`: Capture and retain clipboard history locally with local-first storage and history management.
- `encrypted-cloud-sync`: Encrypt selected clipboard items client-side and sync ciphertext plus minimal metadata to a personal server.
- `device-enrollment-and-trust`: Approve new devices from an already unlocked device and manage trusted device membership in the vault.
- `single-owner-server-bootstrap`: Initialize a server as a single-owner vault backend and prevent use as a shared multi-user service.

### Modified Capabilities

None.

## Impact

- Affects the Tauri client architecture for desktop and mobile packaging, local persistence, clipboard integration, key management, and sync UX.
- Adds a Rust HTTP service backed by PostgreSQL for encrypted item storage, device enrollment, trust management, and sync coordination.
- Introduces cryptographic design work for vault keys, device keys, enrollment approval, and recovery flows.
- Establishes product constraints that influence future API design, deployment model, and review expectations for the open-source repository.
