# Build A Transactional Multi-Output Monitor Planner

## Why This Exists

The current `modify` path still thinks mostly in terms of "change one connector, then rebuild the rest from current state". That is enough for simple one-output edits, but it is the wrong foundation for real `--off`, relative placement, mirroring, and robust rotation reflow.

This note is the architectural prerequisite for the bigger parity features that follow.

## Scope

- introduce an internal planner that starts from the full current monitor state and produces one complete `ApplyMonitorsConfig` payload
- separate planning from CLI parsing so future commands can compose multiple topology changes cleanly
- keep the user-facing `modify` model modern and explicit rather than copying `xrandr --output` chaining
- preserve dry-run and current help/completion behavior while changing the internals

## Acceptance Criteria

- single-output `modify` continues to work through the new planner with no behavioral regressions
- the planner can express monitor removal, monitor repositioning, primary changes, and transform changes together before one apply
- tests cover planner behavior without requiring a live Mutter session
- later notes can build on this planner instead of rewriting ad hoc config assembly again

## Likely Files

- `src/cli/modify/mod.rs`
- `src/display_config/proxied_methods.rs`
- `src/display_config/mod.rs`
- new planner-focused modules under `src/cli/modify/`

## Follow-ups

- `0060_add_real_output_disable_and_absolute_positioning.md`
- `0070_add_relative_placement_and_fix_rotation_reflow.md`
- `0080_add_same_as_clone_group_support_with_clear_mutter_limits.md`
