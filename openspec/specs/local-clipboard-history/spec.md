# local-clipboard-history Specification

## Purpose
TBD - created by archiving change add-private-clipboard-vault. Update Purpose after archive.
## Requirements
### Requirement: Client stores clipboard history locally by default
The client SHALL capture supported clipboard content and store the resulting history in a local embedded database on the device by default. Local storage SHALL not require a cloud server connection.

#### Scenario: Local capture while offline
- **WHEN** the client captures a supported clipboard item while the device has no network connection
- **THEN** the item is stored in local history and remains available for local browsing

#### Scenario: Local history without cloud configuration
- **WHEN** the user has not configured any server connection
- **THEN** the client still provides clipboard capture and local history management

### Requirement: Local history identifies cloud sync state per item
The client SHALL track whether each history item is local-only or selected for cloud sync so the user can distinguish unsynced content from encrypted synced content.

#### Scenario: Unsynced history item
- **WHEN** a newly captured clipboard item has not been selected for cloud sync
- **THEN** the item is shown as local-only in history

#### Scenario: Synced history item
- **WHEN** a history item has been successfully encrypted and uploaded to the personal server
- **THEN** the item is shown as synced in history

### Requirement: User explicitly chooses cloud sync per item
The system SHALL require an explicit user action from the history interface before any clipboard item is synchronized to the server.

#### Scenario: Item remains local without user action
- **WHEN** the user captures clipboard history but does not choose to sync a specific item
- **THEN** the client does not upload that item to the server

#### Scenario: User selects item for sync
- **WHEN** the user chooses to sync a specific history item
- **THEN** the client prepares that item for encrypted cloud synchronization

