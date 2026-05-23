# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.1](https://github.com/myypo/cheers/releases/tag/v0.1.0-alpha.1) - 2026-05-23

### Added

- split CSS bundles
- add form(flatten) attribute
- replace id, form and signal macro API with method API
- send source rust snippet to pi for iterate
- add pi extension
- allow overwriting chromium binary with CHROME_BIN
- add include_js_bundle!
- support datastar attribute modifiers
- use on_* notation for builtin datastar events like intersect
- support custom datastar events with define_events!
- make signals local by default
- add initial subsecond support
- support differing router/handler states for action registration
- get rid of automatic inventory action registration
- remove tracing feature and trace unconditionally
- implement Render<JsSource> for ElementId and FormName
- remove action-mocking feature
- add js macro
- add thirtyfour integration for tests
- replace *_borrow! macros with @& per-expression syntax
- make datastar a regular dir instead of git submodule
- add action mocking for tests
- remove JsSource Context implementation for big ints
- implement Render for Vec in Js context
- add JsSource context and canonical signal-path handling
- make css and svg registration comp-time via inventory
- add PatchSignals
- remove traceparent from track
- change optional prop syntax from (...) to [...]
- add storageless observability/analytics tracking feature
- support svg sprites with include_svg_sprite! macro
- rework scoped_signal as a convention-enforcing proc macro
- rename Refs macro to Cheers
- add support for optional props
- add opengraph support
- add svg support
- support using PatchElements id and selector for multi-target patch
- add cheers skill
- rename Component macro to Refs
- add macros for ids, signals and forms
- make signals, form names and ids API &self based
- remove field based ids
- base component uniqueness on single id field
- generate dot-split nested signals
- output constant instead of closure for signals without ids
- implement Deserialize, PartialEq and PartialOrd for ElementId
- allow using borrowed types in signals
- implement id helper struct
- use id_ prefix instead of _id suffix for id methods
- implement signals helper struct
- implement Deserialize for created Json signals
- use form_ prefix for generated FormNames fields

### Fixed

- avoid ambiguous blob import in the namespace macro code
- switch from SSE to WS for live-reload to avoid browser conn exhaustion
- avoid live-reloading ignored files
- meet cargo-deny demands
- allow clippy::too_many_arguments for __cheers_props
- prevent forcing all components into the prop builder mode

### Other

- add third-party notices
- dedupe signal path code
- improve the readme example
- major pre-release plumbing
- modernize with prek and add new hooks
- share validation element declarations
- add initial rustdoc for public API
- add doc-comments explaining doc hidden declarations
- use rust instead of HTML examples for attribute docs
- make __static constructors const
- use crates directory pattern
# Changelog

All notable changes to Cheers will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
