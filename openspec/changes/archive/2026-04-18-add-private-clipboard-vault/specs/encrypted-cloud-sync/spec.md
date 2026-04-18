## ADDED Requirements

### Requirement: Clipboard items are encrypted client-side before upload
The client SHALL encrypt clipboard content before any synced item is transmitted to the server. The server SHALL receive ciphertext and encrypted key material but SHALL not receive plaintext clipboard content.

#### Scenario: Sync selected item
- **WHEN** the user selects a local history item for cloud sync
- **THEN** the client encrypts the content locally before upload and sends only encrypted payloads to the server

#### Scenario: Server stores encrypted payload
- **WHEN** the server persists a synced clipboard item
- **THEN** the stored record contains ciphertext and sync metadata rather than plaintext content

### Requirement: Synced items can be restored on trusted devices
Any trusted device with access to the vault key material SHALL be able to download and decrypt synced clipboard items from the server.

#### Scenario: Existing trusted device syncs history
- **WHEN** a trusted device requests cloud history after joining the vault
- **THEN** the device can download encrypted items and decrypt them locally

#### Scenario: Untrusted device cannot restore content
- **WHEN** a device has not been approved into the vault
- **THEN** it cannot obtain the key material needed to decrypt synced items

### Requirement: Server filtering relies on minimal metadata
The server SHALL expose sync records using non-content metadata needed for coordination, including timestamps, item type, and origin device, without requiring plaintext access to the clipboard body.

#### Scenario: List synced history by time
- **WHEN** a trusted client requests synced history ordered by time
- **THEN** the server returns encrypted item references ordered by stored timestamp metadata

#### Scenario: Filter by origin device
- **WHEN** a trusted client requests synced history filtered by origin device
- **THEN** the server filters records by device metadata without inspecting clipboard plaintext
