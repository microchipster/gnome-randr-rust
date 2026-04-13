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

## How This Was Addressed

- added `src/cli/modify/planner.rs` with a `MonitorPlanner` that starts from the full current `DisplayConfig` and builds the complete `ApplyMonitorsConfig` payload up front, including mirrored logical monitors with more than one associated connector
- moved `modify` away from the previous ad hoc per-output `ApplyConfig::from(...)` rebuild path so mode, scale, rotation, primary, and future topology changes all target the same planner state before one final apply
- implemented planner operations for mode changes, scale changes, transform changes, primary changes, future-facing position updates, and future-facing output removal so later topology notes can extend the same internal model instead of replacing it again
- kept the current CLI behavior intact: connector resolution, dry-run output, help text, completion behavior, and the existing brightness-after-layout sequencing are unchanged from the user perspective
- added unit tests for full-state planner initialization, composing multiple layout mutations before one apply, and removing outputs from the plan without requiring a live Mutter session

## How To Exercise And Test It

- confirm ordinary single-output dry runs still behave the same through the new planner:
  - `cargo run -- modify --mode 1920x1080 --refresh 60 --dry-run`
  - `cargo run -- modify --scale 2 --dry-run`
  - `cargo run -- modify --primary --dry-run`
- confirm layout and brightness still sequence cleanly together:
  - `cargo run -- modify --mode 1920x1080 --refresh 60 --brightness 1.25 --filter filmic --dry-run`
- confirm query/help/completions still match the existing surface:
  - `cargo run -- modify --help`
  - `cargo run -- __complete "" modify --refresh`
  - `cargo run -- __complete "" modify --scale`
- confirm the internal planner tests pass without a live display server:
  - `cargo test cli::modify::planner`
