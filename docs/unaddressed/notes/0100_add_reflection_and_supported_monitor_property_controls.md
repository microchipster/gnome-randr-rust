# Add Reflection And Supported Monitor Property Controls

## Why This Exists

Beyond rotation and gamma, there are still a few high-value output controls that map reasonably well to Mutter:

- reflection via flipped transforms
- underscanning where supported
- color mode or similar output properties where Mutter exposes them

These are practical parity features and also improve the value of the existing query/property inspection work.

## Scope

- add `modify --reflect x|y|xy|normal` using the existing transform bit model
- expose supported per-monitor properties through explicit flags instead of a generic X11-style `--set`
- start with properties already surfaced by Mutter and query output, especially underscanning and color-mode-like state where applicable
- keep query text and JSON consistent with the new property controls

## Acceptance Criteria

- users can reflect a display through the CLI without abusing raw transform integers
- property flags only appear when they map cleanly to known Mutter semantics
- unsupported properties fail explicitly instead of silently pretending to be generic `xrandr --set`
- tests cover transform serialization and property application paths

## Likely Files

- `src/display_config/logical_monitor.rs`
- `src/cli/modify/mod.rs`
- planner modules introduced by `0050_build_a_transactional_multi_output_monitor_planner.md`
- `src/cli/query.rs`

## Follow-ups

- do not expand this into arbitrary raw property setting
- generic X11-style property plumbing belongs in the backend-limits documentation note instead
