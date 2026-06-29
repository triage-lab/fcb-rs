## ADDED Requirements

### Requirement: Protocol reference docs are accurate and symbol-anchored

The protocol reference documents under `docs/` (`docs/fcb-reference.md`, `docs/fcb-wire-format.md`, `docs/fcb-data-model.md`) SHALL stay consistent with the implemented codec and SHALL reference source code by symbol name (function, type, or constant) rather than by source line number, so the documentation does not drift when code is refactored. Precise values SHALL be anchored to named constants and the frozen golden vectors. These documents SHALL NOT state a license other than the `ECL-2.0` declared in the crate manifests, SHALL NOT describe `pack_case` or `CasePayload` as absent, SHALL describe the capability set consistently with the actual `openspec/specs/` directories, and SHALL scope the manifest/payload superset gap to the reader path because the producer path (`pack_case`) enforces it.

#### Scenario: reference docs survive a code refactor without drifting

- **WHEN** a developer consults the protocol reference docs after a refactor that moves source lines
- **THEN** the docs SHALL still resolve to the named symbols and frozen golden vectors they cite
- **AND** they SHALL state the `ECL-2.0` license, present `pack_case` and `CasePayload` as available, and describe the capability set matching `openspec/specs/`

#### Scenario: reference docs carry no source line-number anchors

- **WHEN** the protocol reference docs reference implementation code
- **THEN** they SHALL identify it by function, type, or constant name rather than by a source line number
