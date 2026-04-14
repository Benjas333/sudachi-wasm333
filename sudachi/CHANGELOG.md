# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog],
and this project adheres to [Semantic Versioning].

## [Unreleased]

- /

## [1.0.4] - 2026-04-13 (v0.6.11)

### Changed

- Synced and merged to upstream sudachi v0.6.11

## [1.0.3] - 2026-02-19 (v0.6.10)

### Added

- justfile for easier building and testing

### Changed

- Merged upstream last commit
- Merged old wasm32 `system_specific_name()` with the new `make_system_specific_name()` from the upstream last commit
- Minor changes on `wasm-pack-inline.mjs`
- Updated `README.md`

## [1.0.2] - 2025-10-03 (v0.6.10)

### Changed

- Package now includes the small dict version, because the full one is 3x larger.

## [1.0.1] - 2025-10-03 (v0.6.10)

### Added

- This changelog xd

### Changed

- Minor README.md changes (probably there will be more)

### Fixed

- Resources dir not included in package (lol)

## [1.0.0] - 2025-10-03 (v0.6.10)

- initial release

<!-- Links -->
[keep a changelog]: https://keepachangelog.com/en/1.0.0/
[semantic versioning]: https://semver.org/spec/v2.0.0.html
