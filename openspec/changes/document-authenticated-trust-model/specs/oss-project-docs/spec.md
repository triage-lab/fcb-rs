## ADDED Requirements

### Requirement: Security policy documents the cryptographic trust boundaries

The `SECURITY.md` SHALL document the container's cryptographic trust boundaries: what the AEAD authenticates — the encrypted payload together with the entire plaintext header and its framing prefix, bound as additional authenticated data — and what remains a deliberate by-design boundary rather than a cryptographic guarantee. The documented by-design boundaries SHALL include at least: the `bundle_hash` is a content address over the plaintext payload and can act as a confirmation oracle for low-entropy payloads; the submission-to-case binding identity is sensitive to any re-pack of the case payload; and the stream manifest's declared record counts are advisory metadata not enforced against the payload. The descriptions SHALL be consistent with the implemented codec behavior and SHALL NOT describe the plaintext header as unauthenticated.

#### Scenario: a reader learns what is and is not cryptographically guaranteed

- **WHEN** a reader opens `SECURITY.md` to understand the threat model
- **THEN** it SHALL state that the plaintext header is authenticated as AEAD additional authenticated data
- **AND** it SHALL list the remaining by-design boundaries, including the bundle_hash confirmation oracle, the re-pack sensitivity of the binding, and the advisory nature of manifest record counts
