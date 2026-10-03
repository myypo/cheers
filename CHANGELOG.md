# Changelog

All notable changes to Cheers will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-alpha.2](https://github.com/myypo/cheers/compare/v0.1.0-alpha.1...v0.1.0-alpha.2) - 2026-10-03

### Added

- add HTTP status to track ([#65](https://github.com/myypo/cheers/pull/65))
- test harness and fmt updates ([#62](https://github.com/myypo/cheers/pull/62))
- add command html attribute ([#59](https://github.com/myypo/cheers/pull/59))
- add SvgSpritePreload ([#57](https://github.com/myypo/cheers/pull/57))
- [**breaking**] add required initially bool argument to !show
- add action options
- add form_selector and form_id methods to actions

### Fixed

- [**breaking**] support `@async` inside `@for`, `@if` and `@match` ([#61](https://github.com/myypo/cheers/pull/61))
- allow `(@&...)` to borrow template-local bindings ([#60](https://github.com/myypo/cheers/pull/60))
- compress no-transform streams

### Other

- compilation speed improvements ([#63](https://github.com/myypo/cheers/pull/63))
- refresh trybuild stderr snapshots
