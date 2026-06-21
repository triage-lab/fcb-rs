## ADDED Requirements

### Requirement: Reference and changelog reflect the authenticated-container model

The `CHANGELOG.md` SHALL record the breaking trust-model hardening as distinct entries: the plaintext header becoming authenticated via AEAD additional authenticated data, the `min_reader` increment that makes pre-authentication readers refuse the new bundles, and the pack-boundary numeric contract that rejects out-of-safe-range integer numbers supplied as plain JavaScript numbers. The cookbook and reference layer SHALL document the case-open content-address verification — the recomputation of `bundle_hash` from the decrypted payload — and its confirmation-oracle caveat for low-entropy payloads, and SHALL document that the submission-to-case binding is invalidated by any re-pack of the case payload. All such prose SHALL follow the documentation language standard.

#### Scenario: the changelog records the breaking trust-model change

- **WHEN** a reader opens `CHANGELOG.md` after this change ships
- **THEN** under an Unreleased or versioned heading it SHALL list the header authentication, the min_reader increment, and the pack-boundary numeric contract as breaking changes

#### Scenario: a consumer finds the content-address and binding caveats in the cookbook

- **WHEN** a developer consults the cookbook for verification and binding behavior
- **THEN** it SHALL describe the case-open bundle_hash recomputation and its confirmation-oracle caveat
- **AND** it SHALL state that re-packing a case payload invalidates prior submission bindings
