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
