## ADDED Requirements

### Requirement: New devices require approval from an unlocked trusted device
The system SHALL require a new device to be approved by an already unlocked trusted device before the new device becomes a trusted member of the vault.

#### Scenario: Successful approval
- **WHEN** a pending device enrollment is approved by an unlocked trusted device
- **THEN** the new device is added to the trusted device set for the vault

#### Scenario: No trusted device approval
- **WHEN** no unlocked trusted device approves the pending enrollment
- **THEN** the new device remains untrusted and cannot access cloud data

### Requirement: Enrollment codes are short-lived join initiators
The system SHALL use a time-limited enrollment code only to associate a pending join request with the existing vault. Possession of the code alone SHALL NOT grant trusted device status.

#### Scenario: Valid enrollment code begins pending join
- **WHEN** a new device submits a valid unexpired enrollment code with its device public key
- **THEN** the server creates or updates a pending enrollment request for trusted-device review

#### Scenario: Expired code is rejected
- **WHEN** a new device submits an expired enrollment code
- **THEN** the server rejects the join attempt and does not create a trusted session

### Requirement: Trusted devices receive wrapped key material
The system SHALL deliver vault key material to a newly approved device only after an existing trusted device wraps that material for the new device public key.

#### Scenario: Approval distributes wrapped vault key
- **WHEN** a trusted device approves a pending device
- **THEN** the approving client uploads vault key material encrypted for the new device public key

#### Scenario: Pending device completes activation
- **WHEN** the approved device receives wrapped vault key material for its own private key
- **THEN** it can activate as a trusted device and decrypt synced clipboard items locally

### Requirement: Users can manage the trusted device list
The client SHALL present the current trusted device set and allow the owner to revoke a device from future cloud access.

#### Scenario: Revoke old device
- **WHEN** the user revokes a trusted device
- **THEN** the revoked device loses future access to server sessions and new sync state

#### Scenario: View trusted devices
- **WHEN** the user opens device management
- **THEN** the client shows the known trusted devices for the vault

### Requirement: Recovery path exists when no trusted device remains
The system SHALL provide a recovery mechanism that allows the owner to re-establish access to the vault without requiring the server to decrypt stored clipboard content.

#### Scenario: Recovery after device loss
- **WHEN** the owner has lost access to all trusted devices but possesses the recovery secret
- **THEN** the owner can start a recovery flow to re-establish trusted access

#### Scenario: Recovery secret unavailable
- **WHEN** the owner has lost all trusted devices and does not possess the recovery secret
- **THEN** the system cannot grant access to encrypted clipboard content through the server alone
