## Context

This change defines the first architecture for a private clipboard product intended for one person using multiple devices. The product must remain open-source and self-hostable while ensuring that a deployed server instance is not a shared multi-user service. The client is packaged with Tauri for desktop and mobile targets, and the server is implemented as a Rust HTTP API backed by PostgreSQL.

The core constraint is strong privacy by default. Clipboard content is captured and stored locally first, and cloud sync is opt-in per history item. The server must not be able to read clipboard content, which means sync, device enrollment, and recovery must all work with end-to-end encryption and device-based trust rather than conventional username/password authentication.

## Goals / Non-Goals

**Goals:**
- Provide one consistent client product across device types, with platform differences handled below the product layer.
- Store clipboard history locally by default using an embedded database suitable for offline, single-user operation.
- Allow users to explicitly sync selected clipboard items to a personal server using client-side encryption.
- Allow new devices to join only after approval from an already unlocked trusted device.
- Keep each server instance bound to a single owner vault instead of supporting multi-user tenancy.
- Support a recovery path that does not require the server to decrypt user data.

**Non-Goals:**
- Multi-user workspaces, team sharing, or delegated administration.
- Server-side plaintext indexing, moderation, or content inspection.
- Automatic sync of every clipboard event in the first version.
- Strongly consistent cross-device full-text search over encrypted cloud data in the first version.
- Separate product roles such as a dedicated approval-only device or admin console.

## Decisions

### Local persistence uses SQLite, not PostgreSQL

The client SHALL use SQLite for local clipboard history, device metadata, and cached encrypted sync state. SQLite fits the local-first and cross-platform packaging constraints better than a bundled PostgreSQL instance, and it keeps the desktop and mobile installation model lightweight.

Alternative considered:
- Local PostgreSQL: rejected because it increases operational weight, complicates packaging, and weakens offline-first ergonomics for a single-user client.

### The server is a single-owner vault backend

Each deployed server SHALL be initialized once for a single owner vault. The bootstrap device becomes the first trusted device for that vault, and the API model supports adding devices to that vault rather than creating multiple user accounts.

Alternative considered:
- Multi-user account system: rejected for the first version because it adds tenancy, permission, and lifecycle complexity without supporting the product goal of one personal instance per person.

### Clipboard content is encrypted before it leaves the client

The system SHALL encrypt synced clipboard content on the client with a vault content key. The server stores only ciphertext, encrypted key material, and minimal metadata needed for synchronization and device management.

Alternative considered:
- Server-readable content with transport encryption only: rejected because it violates the explicit privacy requirement and makes self-hosting trust-dependent rather than cryptography-dependent.

### Device trust is based on key approval, not code-only login

Every device SHALL generate its own keypair during setup. A short-lived enrollment code is used only to start a join request; an existing unlocked trusted device must approve the new device and share wrapped vault key material for the new device public key. This keeps the approval decision bound to device cryptography rather than to possession of a short numeric code.

Alternative considered:
- Code-only enrollment: rejected because possession of the code alone would effectively become the authentication factor and would not establish durable device identity.

### Recovery is separate from daily enrollment

The system SHALL generate a recovery secret during initialization so the owner can regain access when no trusted device remains. Recovery is a break-glass path for re-establishing trust and rewrapping vault keys, not a normal shortcut around device approval while trusted devices are available.

Alternative considered:
- No recovery path: rejected because a strong privacy model without recovery would make total device loss equivalent to permanent data loss for many users.

### Cloud sync is explicit and selective

The first version SHALL sync only clipboard items the user explicitly marks for cloud sync from history. This reduces accidental data leakage, simplifies conflict behavior, and aligns with the expectation that cloud storage is optional rather than the default destination.

Alternative considered:
- Always-on automatic sync: rejected for the first version because it expands privacy risk and increases edge cases around sensitive one-time content.

### Search remains local-first in the first version

The server SHALL support filtering encrypted records by non-sensitive metadata such as timestamps, origin device, and content class, but it SHALL not provide plaintext full-text search. Full-text search is performed against locally available decrypted history on the client.

Alternative considered:
- Encrypted searchable index: deferred because it adds cryptographic and product complexity that is not required for the initial capability set.

## Risks / Trade-offs

- [Clipboard APIs differ across desktop and mobile] -> Constrain the first version to one shared product with platform-specific capability fallbacks and document that background capture behavior may vary by OS.
- [End-to-end encryption complicates debugging and support] -> Keep server-side metadata explicit enough to diagnose enrollment and sync state without exposing content.
- [Recovery secrets can be mishandled by users] -> Make recovery setup explicit during bootstrap and require confirmation that the secret has been recorded.
- [Selective sync may disappoint users expecting seamless replication] -> Keep the user experience clear that local capture is automatic but cloud sync is opt-in per item.
- [Single-owner server limits future sharing use cases] -> Treat personal vault scope as a deliberate product boundary for the first architecture rather than a temporary omission.

## Migration Plan

This is a new product capability set with no existing production deployment to migrate. Implementation can proceed by creating the client and server foundations in parallel, then integrating the enrollment and encrypted sync flows.

If future revisions broaden the deployment model, the server bootstrap state must remain versioned so existing single-owner instances can continue operating without data migration surprises.

## Open Questions

- Whether the first version should support only text clipboard items or also images and file references in encrypted sync.
- Whether recovery can add a replacement device directly or should require first re-establishing a trusted device and then following the standard enrollment flow.
- How much metadata can be exposed to the server for sync usability without violating privacy expectations.
