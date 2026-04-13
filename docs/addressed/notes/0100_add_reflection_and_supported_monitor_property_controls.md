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

## How This Was Addressed

- added `modify --reflect normal|x|y|xy`, composing xrandr-style reflection names onto Mutter's flipped transform model instead of exposing raw transform integers
- added typed `ColorMode` support with `modify --color-mode default|bt2100`, using the per-monitor property map inside `ApplyMonitorsConfig` rather than inventing generic X11-style property setting
- extended the planner and apply payload so monitor tuples now carry typed writable properties, currently `ColorMode`
- updated `query` text and JSON to expose typed `rotation`, `reflection`, `color_mode`, `supported_color_modes`, and `is_underscanning` fields while keeping the raw property map available for deeper inspection
- kept writable underscanning out of the CLI on purpose because the current Mutter/gdctl-supported write surface could not be justified beyond query visibility
- added tests covering color-mode property serialization and typed query output alongside the existing modify/completion paths

## How To Exercise And Test It

- preview reflection changes:
  - `cargo run -- modify --reflect x --dry-run`
  - `cargo run -- modify --reflect xy --dry-run`
- preview a typed color-mode change on hardware that exposes it:
  - `cargo run -- modify --color-mode default --dry-run`
  - `cargo run -- modify --color-mode bt2100 --dry-run`
- inspect typed query output:
  - `cargo run -- query --summary`
  - `cargo run -- query --json | jq '.logical_monitors[] | {rotation, reflection, monitors}'`
  - `cargo run -- query --json | jq '.monitors[] | {connector, color_mode, supported_color_modes, is_underscanning}'`
- inspect the raw property map alongside the typed fields:
  - `cargo run -- query --properties`
- targeted tests for single-monitor environments:
  - `cargo test apply_monitor_serializes_color_mode_property`
  - `cargo test cli::query::tests::json_output_uses_documented_schema`
  - `cargo test cli::query::tests::summary_output_reports_enabled_state`
