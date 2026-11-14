# Changelog

All notable changes to Cheers will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-alpha.2](https://github.com/myypo/cheers/compare/v0.1.0-alpha.1...v0.1.0-alpha.2) - 2026-11-14

### Added

- *(test)* hold action requests and capture tracked items in browser tests ([#84](https://github.com/myypo/cheers/pull/84))
- [**breaking**] render style element bodies as CSS ([#82](https://github.com/myypo/cheers/pull/82))
- *(test)* reuse Chromium browsers between test apps through CHEERS_TEST_BROWSER_POOL ([#79](https://github.com/myypo/cheers/pull/79))
- add `datastar_object!` and `..` spreads in Datastar attribute objects ([#75](https://github.com/myypo/cheers/pull/75))
- [**breaking**] render script element bodies as JavaScript ([#74](https://github.com/myypo/cheers/pull/74))
- navigate the browser on redirects answering Datastar requests ([#72](https://github.com/myypo/cheers/pull/72))
- [**breaking**] add `EventStream::from_stream` and `EventStream::spawn` ([#71](https://github.com/myypo/cheers/pull/71))
- [**breaking**] add `EventBatch` and rename the event types ([#70](https://github.com/myypo/cheers/pull/70))
- add `EventSender::closed` ([#68](https://github.com/myypo/cheers/pull/68))
- add HTTP status to track ([#65](https://github.com/myypo/cheers/pull/65))
- test harness and fmt updates ([#62](https://github.com/myypo/cheers/pull/62))
- add command html attribute ([#59](https://github.com/myypo/cheers/pull/59))
- add SvgSpritePreload ([#57](https://github.com/myypo/cheers/pull/57))
- [**breaking**] add required initially bool argument to !show
- add action options
- add form_selector and form_id methods to actions

### Fixed

- report syntax errors inside nested groups of a data attribute ([#83](https://github.com/myypo/cheers/pull/83))
- replace hung pooled test browsers and format or-patterns instead of panicking ([#80](https://github.com/myypo/cheers/pull/80))
- escape script bodies across dynamic parts and add escaping property tests ([#77](https://github.com/myypo/cheers/pull/77))
- finish Datastar requests answered with a script ([#73](https://github.com/myypo/cheers/pull/73))
- [**breaking**] reject `Option` in JavaScript contexts and extra `@async` fallback nodes ([#69](https://github.com/myypo/cheers/pull/69))
- quit the test browser when a test built with Cranelift panics ([#66](https://github.com/myypo/cheers/pull/66))
- [**breaking**] support `@async` inside `@for`, `@if` and `@match` ([#61](https://github.com/myypo/cheers/pull/61))
- allow `(@&...)` to borrow template-local bindings ([#60](https://github.com/myypo/cheers/pull/60))
- compress no-transform streams
- allow dead_code on the generated required-props struct for closed-world lints ([#78](https://github.com/myypo/cheers/pull/78))
- *(fmt)* lay control-flow patterns and expressions out against the configured line length ([#81](https://github.com/myypo/cheers/pull/81))
- *(fmt)* keep code after line comments and comments around nested macros ([#76](https://github.com/myypo/cheers/pull/76))

### Other

- compilation speed improvements ([#63](https://github.com/myypo/cheers/pull/63))
- refresh trybuild stderr snapshots
