# Add Software Gamma Controls

## Why This Exists

Software brightness is now implemented with exact LUT preservation, but the corresponding `xrandr`-style gamma controls are still missing. Adding gamma is the most natural next output-control feature because it can reuse the same gamma-ramp access path and state-management discipline.

## Scope

- add `modify --gamma R[:G:B]`
- share the same per-output software color baseline logic already used for brightness so color managers and Night Light are preserved instead of overwritten blindly
- define how gamma interacts with brightness and filter state so the resulting behavior is predictable and queryable
- extend `query` text and JSON output if needed so current managed color state is inspectable

## Acceptance Criteria

- users can apply per-channel gamma corrections through `modify`
- repeated absolute gamma changes do not compound unexpectedly when the current ramp still matches the last tool-managed state
- applying gamma after another tool changed the ramp preserves that new baseline instead of reconstructing a lossy curve
- tests cover state matching and gamma application math

## Likely Files

- `src/cli/brightness.rs`
- `src/cli/modify/mod.rs`
- `src/cli/query.rs`
- `src/display_config/proxied_methods.rs`

## Follow-ups

- reflection and monitor-property mutation belong in `0100_add_reflection_and_supported_monitor_property_controls.md`

## How This Was Addressed

- generalized the old brightness-only saved state into a managed software-color state that stores brightness, brightness filter, per-channel gamma adjustment, and the preserved baseline LUT
- added `modify --gamma R[:G:B]`, following `xrandr` semantics where a single value applies to all three channels
- implemented exact per-channel LUT gamma application in `Gamma::apply_gamma_adjustment(...)` and composed software color as gamma first, then brightness/filter second
- reused the existing baseline-matching discipline so repeated absolute brightness and gamma operations do not compound while the current LUT still matches the last tool-managed state, but external LUT changes are still adopted as the new baseline
- extended `query` text and JSON output to report both `software_brightness` and `software_gamma`, and bumped the JSON schema to version `4`
- added unit tests for gamma parsing, LUT gamma math, combined software-color ordering, saved-state compatibility, state matching, and query output

## How To Exercise And Test It

- preview a software gamma change:
  - `cargo run -- modify --gamma 1.1:1.0:0.9 --dry-run`
- preview combined gamma and brightness:
  - `cargo run -- modify --brightness 1.25 --filter gamma --gamma 1.1 --dry-run`
- inspect the current managed software color state in text:
  - `cargo run -- query --summary`
- inspect the current managed software color state in JSON:
  - `cargo run -- query --json | jq '.monitors[0] | {software_brightness, software_gamma}'`
- verify targeted regression tests without relying on a multi-monitor setup:
  - `cargo test gamma_adjustment_changes_channels_independently`
  - `cargo test software_color_applies_gamma_before_brightness`
  - `cargo test matching_state_reports_saved_gamma_and_brightness`
