# Add Richer query Views And Property Inspection

## Why This Exists

Current `query` output is good, but it still lacks some of the inspection affordances people used from `xrandr`, especially property-heavy views and monitor-focused summaries.

This is a good bounded slice because the data is already available from Mutter and much of it is already present in JSON output.

## Scope

- add `query --properties` / `--prop` to surface monitor properties in the text UI
- add `query --verbose` as a more detailed inspection mode rather than a no-op alias
- add `query --listmonitors` and `query --listactivemonitors` views oriented around logical monitors instead of full mixed dumps
- make sure the text and JSON outputs agree on naming and meaning for the shared fields

## Acceptance Criteria

- users can inspect monitor properties such as underscanning-related state from the CLI without switching to JSON
- `--listmonitors` and `--listactivemonitors` provide concise monitor-centric output rather than the default full dump
- the new flags are documented in help text and README examples
- tests cover formatting and flag interactions

## Likely Files

- `src/cli/query.rs`
- `src/cli/mod.rs`
- `README.md`

## Follow-ups

- do not add mutation behavior to `query`
- output-control property mutation belongs in `0100_add_reflection_and_supported_monitor_property_controls.md`

## How This Was Addressed

- added `query --properties` with `--prop` alias so the text UI can surface raw Mutter property maps for displays, logical monitors, physical monitors, and modes when they are present
- added `query --verbose` as a JSON-aligned detailed text view with explicit field names such as `layout_mode`, `display_name`, `software_brightness`, `is_current`, and `is_preferred`
- added `query --listmonitors` and `query --listactivemonitors` to print concise xrandr-style logical-monitor lists showing geometry, primary status, and associated connectors
- extended `query --json` to schema version `2`, adding optional raw `properties` maps at the display, logical-monitor, monitor, and mode levels so the text and JSON inspection surfaces share the same property names and meanings
- added unit tests for JSON schema content, property formatting, xrandr-style list view formatting, and flag-interaction validation

## How To Exercise And Test It

- inspect the concise logical-monitor list:
  - `cargo run -- query --listmonitors`
- inspect the active logical-monitor list:
  - `cargo run -- query --listactivemonitors`
- inspect detailed text output for one connector:
  - `cargo run -- query eDP-1 --verbose`
- inspect raw properties in the text UI:
  - `cargo run -- query --properties`
- inspect raw properties in JSON:
  - `cargo run -- query --json | jq '.monitors[] | {connector, properties}'`
- confirm flag conflicts are rejected cleanly:
  - `cargo run -- query --summary --json`
  - `cargo run -- query --properties --listmonitors`
