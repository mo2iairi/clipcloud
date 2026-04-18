# single-owner-server-bootstrap Specification

## Purpose
TBD - created by archiving change add-private-clipboard-vault. Update Purpose after archive.
## Requirements
### Requirement: Server initializes a single owner vault
The server SHALL support a bootstrap flow that creates exactly one owner vault for a deployment and associates the first approved device with that vault.

#### Scenario: First-time bootstrap
- **WHEN** a fresh server instance is initialized
- **THEN** it creates a single owner vault state and records the first device as the bootstrap trusted device

#### Scenario: Existing vault prevents second bootstrap
- **WHEN** a server that already has an initialized owner vault receives another bootstrap request
- **THEN** the server rejects the request as already initialized

### Requirement: Server does not provide general multi-user registration
The server SHALL expose device enrollment and sync APIs for the existing owner vault but SHALL NOT expose an API for creating independent second user accounts on the same deployment.

#### Scenario: Client joins existing vault
- **WHEN** a new device starts enrollment against an initialized server
- **THEN** the server treats the request as an attempt to join the existing owner vault

#### Scenario: Second user account creation is unavailable
- **WHEN** a client attempts to create a separate account on an initialized server
- **THEN** the server does not offer or complete a second user registration flow

### Requirement: Server stores encrypted sync and trust metadata only
The server SHALL persist encrypted clipboard payloads, wrapped key material, device trust state, and synchronization metadata sufficient to coordinate clients without requiring plaintext access.

#### Scenario: Store encrypted clipboard record
- **WHEN** a trusted device uploads a synced clipboard item
- **THEN** the server stores the encrypted payload together with synchronization metadata

#### Scenario: Store trust metadata
- **WHEN** a trusted device approves or revokes another device
- **THEN** the server stores the resulting trust state and related device metadata

